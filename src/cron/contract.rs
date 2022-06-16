use std::str::FromStr;

use crate::{
    context, contract_gateway,
    state::{self, ValidatorRole},
};

use super::{arweave::ArweaveError, http, CronJobError};

use bundlr_contracts_validators::{
    slashing::Proposal as SlashProposal,
    slashing::{Vote, Voting},
    Address, Epoch, State as ContractState,
};

pub async fn check_contract_updates<Context, HttpClient>(ctx: &Context) -> Result<(), CronJobError>
where
    Context: context::ArweaveAccess
        + context::ValidatorAddressAccess
        + contract_gateway::ContractGatewayAccess
        + http::ClientAccess<HttpClient>
        + state::ValidatorStateAccess,
    HttpClient: http::Client<Request = reqwest::Request, Response = reqwest::Response>,
{
    let contract_gateway = ctx.contract_gateway();

    let state = contract_gateway
        .get_current_state(ctx)
        .await
        .map_err(CronJobError::ContractGatewayError)?;

    if let Some((new_epoch, new_role)) = check_for_epoch_update(ctx, &state).await {
        let state = ctx.get_validator_state();
        state.set_current_epoch(new_epoch.seq);
        state.set_role(new_role);
    }

    if let Some(new_slash_proposals) = check_for_slash_proposals(ctx, &state).await {
        for proposal in new_slash_proposals {
            let is_valid = is_valid_proposal(ctx, proposal)
                .await
                .map_err(CronJobError::ArweaveError)?;

            let vote = if is_valid { Vote::For } else { Vote::Against };
            contract_gateway
                .vote_for_proposal(ctx, proposal, vote)
                .await
                .map_err(CronJobError::ContractGatewayError)?;
        }
    }

    Ok(())
}

async fn check_for_epoch_update<'a, Context>(
    ctx: &Context,
    state: &'a ContractState,
) -> Option<(&'a Epoch, ValidatorRole)>
where
    Context: state::ValidatorStateAccess + context::ValidatorAddressAccess,
{
    let (current_block_height, current_epoch) = {
        let state = ctx.get_validator_state();
        (state.current_block(), state.current_epoch())
    };

    if state.epoch.seq > current_epoch && state.epoch.height <= current_block_height {
        let validator_address = Address::from_str(ctx.get_validator_address()).unwrap();
        let role = if state.nominated_validators.contains(&validator_address) {
            ValidatorRole::Cosigner
        } else {
            ValidatorRole::Idle
        };
        Some((&state.epoch, role))
    } else {
        None
    }
}

async fn check_for_slash_proposals<'a, Context>(
    ctx: &Context,
    state: &'a ContractState,
) -> Option<Vec<&'a SlashProposal>>
where
    Context: context::ValidatorAddressAccess,
{
    let own_address = Address::from_str(ctx.get_validator_address()).unwrap();
    let new_proposals: Vec<&SlashProposal> = state
        .slash_proposals
        .iter()
        .filter(|proposal| {
            if proposal.0 != &own_address {
                match &(proposal.1 .4) {
                    Voting::Closed {
                        votes: _,
                        final_vote: _,
                    } => false,
                    Voting::Open(votes) => !votes.contains_key(&own_address),
                }
            } else {
                false
            }
        })
        .map(|proposal| &(proposal.1 .0))
        .collect();

    if new_proposals.is_empty() {
        None
    } else {
        Some(new_proposals)
    }
}

async fn is_valid_proposal<Context>(
    _ctx: &Context,
    _proposal: &SlashProposal,
) -> Result<bool, ArweaveError> {
    // TODO: implement actual logic for checking if the proposal is valid
    Ok(true)
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use crate::{
        context::test_utils::test_context_with_http_client,
        http::reqwest::mock::MockHttpClient,
        key_manager::{
            test_utils::{test_keys, to_address, validator_key},
            KeyManager,
        },
        state::ValidatorStateAccess,
    };
    use bundlr_contracts_validators::{
        slashing::{Proposal, Vote, Voting},
        Address, Epoch, State, Validator,
    };
    use futures::executor::LocalPool;
    use http::Method;
    use reqwest;

    use super::check_contract_updates;

    fn create_contract_state(
        validators: HashMap<Address, Validator>,
        nominated_validators: Vec<Address>,
    ) -> State {
        State {
            bundler: "bundler_address".try_into().unwrap(),
            bundlers_contract: "bundlers_contract_address".try_into().unwrap(),
            epoch: Epoch {
                seq: 1,
                tx: "tx1".try_into().unwrap(),
                height: 1,
            },
            epoch_duration: 2,
            minimum_stake: 1.into(),
            token: "token_contract_address".try_into().unwrap(),
            max_num_nominated_validators: 10,
            validators,
            nominated_validators,
            slash_proposal_lifetime: 10,
            slash_proposals: HashMap::new(),
        }
    }

    #[test]
    fn epoch_update_before_activation_yields_no_change() {
        let (key_manager, _bundle_pvk) = test_keys();
        let validator_address: Address = key_manager.validator_address().try_into().unwrap();
        let validators: HashMap<Address, Validator> = HashMap::from([(
            validator_address.clone(),
            Validator {
                address: validator_address.clone(),
                url: "https://validator1.example.com".parse().unwrap(),
                stake: 1.into(),
            },
        )]);
        let nominated_validators = validators.keys().cloned().collect();
        let contract_state = create_contract_state(validators, nominated_validators);

        let client = {
            MockHttpClient::new(|a: &reqwest::Request, b: &reqwest::Request| a.url() == b.url())
                .when(|req: &reqwest::Request| {
                    let url = "http://localhost:3000/validators/state";
                    req.method() == Method::GET && &req.url().to_string() == url
                })
                .then(move |_: &reqwest::Request| {
                    let initial_contract_state = serde_json::to_string(&contract_state).unwrap();
                    http::response::Builder::new()
                        .status(200)
                        .body(initial_contract_state)
                        .map(|res| reqwest::Response::from(res))
                        .unwrap()
                })
        };

        let ctx = test_context_with_http_client(key_manager, client);

        let mut rt = LocalPool::new();
        rt.run_until(check_contract_updates(&ctx)).unwrap();

        assert_eq!(ctx.get_validator_state().current_epoch(), 0);
    }

    #[test]
    fn epoch_update_at_the_activation_block_height_yields_epoch_being_updated() {
        let (key_manager, _bundle_pvk) = test_keys();
        let validators: HashMap<Address, Validator> = HashMap::new();
        let nominated_validators = validators.keys().cloned().collect();
        let contract_state = create_contract_state(validators, nominated_validators);

        let client = {
            MockHttpClient::new(|a: &reqwest::Request, b: &reqwest::Request| a.url() == b.url())
                .when(|req: &reqwest::Request| {
                    let url = "http://localhost:3000/validators/state";
                    req.method() == Method::GET && &req.url().to_string() == url
                })
                .then(move |_: &reqwest::Request| {
                    let initial_contract_state = serde_json::to_string(&contract_state).unwrap();
                    http::response::Builder::new()
                        .status(200)
                        .body(initial_contract_state)
                        .map(|res| reqwest::Response::from(res))
                        .unwrap()
                })
        };

        let ctx = test_context_with_http_client(key_manager, client);
        ctx.get_validator_state().set_current_block(1);

        let mut rt = LocalPool::new();
        rt.run_until(check_contract_updates(&ctx)).unwrap();

        assert_eq!(ctx.get_validator_state().current_epoch(), 1);
    }

    #[test]
    fn new_and_valid_slash_proposal_yields_call_to_vote_for_the_proposal() {
        let (key_manager, _bundle_pvk) = test_keys();
        let validator_address: Address = key_manager.validator_address().try_into().unwrap();
        let validators: HashMap<Address, Validator> = HashMap::from([(
            validator_address.clone(),
            Validator {
                address: validator_address.clone(),
                url: "https://validator1.example.com".parse().unwrap(),
                stake: 1.into(),
            },
        )]);
        let nominated_validators = validators.keys().cloned().collect();
        let mut contract_state = create_contract_state(validators, nominated_validators);

        let validator_address_2: Address = {
            let jwk = validator_key();
            to_address(&jwk).unwrap().as_str().try_into().unwrap()
        };

        {
            let validator = validator_address_2.clone();
            let proposal = Proposal {
                id: "missing_tx".try_into().unwrap(),
                size: 1,
                fee: 1,
                currency: "BTC".to_string(),
                block: 1,
                validator: validator.to_string(),
                signature: "not_really_validated_here".to_string(),
            };
            contract_state.slash_proposals.insert(
                validator.clone(),
                (
                    proposal,
                    validator.clone(),
                    2,
                    "proposal_tx".try_into().unwrap(),
                    Voting::Open(HashMap::from([(validator, Vote::For)])),
                ),
            );
        }
        let client = {
            MockHttpClient::new(|a: &reqwest::Request, b: &reqwest::Request| a.url() == b.url())
                .when(|req: &reqwest::Request| {
                    let url = "http://localhost:3000/validators/state";
                    req.method() == Method::GET && &req.url().to_string() == url
                })
                .then(move |_: &reqwest::Request| {
                    let initial_contract_state = serde_json::to_string(&contract_state).unwrap();
                    http::response::Builder::new()
                        .status(200)
                        .body(initial_contract_state)
                        .map(|res| reqwest::Response::from(res))
                        .unwrap()
                })
                .when(|req: &reqwest::Request| {
                    let url = "http://localhost:3000/validators/vote";
                    req.method() == Method::POST && &req.url().to_string() == url
                })
                .then(|_: &reqwest::Request| {
                    // TODO: vote response
                    http::response::Builder::new()
                        .status(200)
                        .body(r#"{"status":"OK"}"#)
                        .map(reqwest::Response::from)
                        .unwrap()
                })
        };

        let ctx = test_context_with_http_client(key_manager, client.clone());
        ctx.get_validator_state().set_current_block(1);

        let mut rt = LocalPool::new();
        rt.run_until(check_contract_updates(&ctx)).unwrap();

        assert_eq!(ctx.get_validator_state().current_epoch(), 1);

        // Drop context to make sure it's not in use anymore
        // and there are no shared references to the client.
        // This is needed so that we can call verify.
        drop(ctx);

        client.verify(|interactions| {
            assert_eq!(interactions.len(), 2);
        });
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn new_but_invalid_slash_proposal_yields_call_to_vote_against_the_proposal() {
        todo!()
    }
}
