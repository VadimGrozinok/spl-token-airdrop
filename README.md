# Metaplex collection airdrop CLI

This CLI was created to make collection holders snapshot and then do SPL tokens airdrop.

Before use CLI we have to build it first: `cargo build`

Then we can run this command and see what commands are available: `./target/debug/collection-airdrop`

Output will be like:

<code>
Airdrop cli 0.1.0

CLI sub-commands

USAGE:

    collection-airdrop [OPTIONS] <SUBCOMMAND>

OPTIONS:

    -h, --help                       Print help information

        --payer-keypair <BASE 58>    Keypair in base 58 encoding [default: ]

        --sleep <MILLISECONDS>       Time to sleep between RPC requests [default: 700]

        --url <URL>                  RPC endpoint [default: https://api.mainnet-beta.solana.com]

    -V, --version                    Print version information

SUBCOMMANDS:

    airdrop               
    help                  Print this message or the help of the given subcommand(s)
    make-fake-snapshot    
    make-snapshot  
</code>

## Commands description

1. **make-snapshot** - this command uses get_program_accounts(GPA) request to get list of pointed collection holders.

Command example:

        ./target/debug/collection-airdrop make-snapshot --collection <KEY> --collection-offset 402 --output-file ./snapshot.json

Pay attention that collection-offset argument can be different such as it depends on data saved in Metadata account.

As a result we'll get snapshot.json file with `<wallet_address>: <amount of tokens from collection wallet holds>`


2. **airdrop** - this command runs a loop with `spl_token::mint_to` instructions. Also CLI will create new associated token account for every wallet.

Command example:

        ./target/debug/collection-airdrop --payer-keypair <PRIV_KEY_AS_BASE58_STR> airdrop --mint <MINT_PUB_KEY> --holders-list ./snapshot.json --one-to-wallet false

One-to-wallet argument tells CLI whether amount of minted SPL tokens should be equal to amount of NFTs from collection wallet has or not.

3. **make-fake-snapshot** - it's just a test command so we can see what data we will receive after **make-snapshot** launch

Command example:

        ./target/debug/collection-airdrop make-fake-snapshot --amount-of-holders 5 --output-file ./fake-holders.json