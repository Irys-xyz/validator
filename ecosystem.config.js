module.exports = {
    apps: [
        {
            "name": "Validator",
            "script": "./target/release/validator",
            "exec": "none",
            "exec_mode": "fork",
            "env": {
                "PORT": "10000",
                "kill_timeout": 120000
            }
        },
        {
            "name": "Contract Gateway",
            "script": "./contracts/gateway/dist/gateway/src/index.js"
        }
    ]
}