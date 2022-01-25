import Bundlr from "@bundlr-network/client";

const bundlr = new Bundlr("https://dev1.bundlr.network", "solana", "<prviate-key>");

const transaction = await bundler.createTransaction("Hello World!");
await transaction.sign();
await transaction.upload();