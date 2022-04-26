use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[clap(name = "Airdrop cli")]
#[clap(version, author)]
pub struct CliArgs {
    /// RPC endpoint.
    #[clap(long, long, default_value_t = String::from("https://api.mainnet-beta.solana.com"), value_name = "URL")]
    pub url: String,

    /// Keypair in base 58 encoding
    #[clap(long, default_value_t = String::from(""), value_name = "BASE 58")]
    pub payer_keypair: String,

    /// Time to sleep between RPC requests
    #[clap(long, default_value_t = 700, value_name = "MILLISECONDS")]
    pub sleep: u64,

    #[clap(subcommand)]
    pub command: Commands,
}

/// CLI sub-commands.
#[derive(Subcommand, Debug)]
pub enum Commands {
    ///
    MakeSnapshot {
        #[clap(long, value_name = "FILE")]
        output_file: String,
        #[clap(long, value_name = "PUBKEY")]
        collection: String,
        #[clap(long, value_name = "NUMBER")]
        collection_offset: usize,
    },
    ///
    Airdrop {
        #[clap(long, value_name = "PUBKEY")]
        mint: String,
        #[clap(long, value_name = "FILE")]
        holders_list: String,
        /// If set to true CLI will mint one SPL token to user wallet and doesn't matter how many tokens from collection he has
        #[clap(long, value_name = "BOOL")]
        one_to_wallet: bool,
    },
    ///
    MakeFakeSnapshot {
        #[clap(long, value_name = "NUMBER")]
        amount_of_holders: u64,
        #[clap(long, value_name = "FILE")]
        output_file: String,
    },
}
