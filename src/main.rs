use serde::{Deserialize, Serialize};
use solana_account_decoder::{UiAccountEncoding, UiDataSliceConfig};
use solana_client::{
    rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType},
};
use solana_sdk::{
    account::Account,
    hash::Hash,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};

use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::str::FromStr;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;
use std::{thread, time};

use clap::Parser;
use cli_args::{CliArgs, Commands};

mod cli_args;

#[derive(Serialize, Deserialize)]
pub struct Holders {
    pub list: HashMap<String, u32>,
}

#[derive(Serialize, Deserialize)]
pub struct MintStatus {
    pub list: HashMap<String, MintTxInfo>,
}

#[derive(Serialize, Deserialize)]
pub struct MintTxInfo {
    pub signature: Option<String>,
    pub status: bool,
}

fn get_program_accounts(
    rpc_client: &RpcClient,
    program_id: &Pubkey,
    memcmps: Vec<RpcFilterType>,
) -> Vec<(Pubkey, Account)> {
    let config = RpcProgramAccountsConfig {
        filters: Some(memcmps),
        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            data_slice: Some(UiDataSliceConfig {
                offset: 33,
                length: 32,
            }),
            commitment: None,
        },
        with_context: Some(false),
    };

    rpc_client
        .get_program_accounts_with_config(program_id, config)
        .unwrap()
}

async fn get_nft_holder(
    mint: Pubkey,
    channel_sender: Sender<Pubkey>,
    url: String,
    time_to_sleep: u64,
) {
    let sleep_time = time::Duration::from_millis(time_to_sleep);

    thread::sleep(sleep_time);

    let timeout = Duration::from_secs(10000000);
    let rpc_client = RpcClient::new_with_timeout(&url, timeout);

    let memcmp = RpcFilterType::Memcmp(Memcmp {
        offset: 0,
        bytes: MemcmpEncodedBytes::Base64(mint.to_string()),
        encoding: None,
    });

    let filters = vec![memcmp, RpcFilterType::DataSize(165)];

    let config = RpcProgramAccountsConfig {
        filters: Some(filters),
        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            data_slice: Some(UiDataSliceConfig {
                offset: 32,
                length: 32,
            }),
            commitment: None,
        },
        with_context: Some(false),
    };

    let mut requesting = true;

    while requesting {
        let token_acc =
            rpc_client.get_program_accounts_with_config(&spl_token::id(), config.clone());

        if let Ok(holder_key) = token_acc {
            let token_owner = Pubkey::new(holder_key[0].1.data.as_ref());

            channel_sender.send(token_owner).unwrap();

            requesting = false;
        }

        thread::sleep(sleep_time);
    }
}

async fn airdrop(
    mint_key: Pubkey,
    url: String,
    holders: Holders,
    payer: String,
    one_to_wallet: bool,
    sleep: u64,
) {
    let (tx, rx): (
        Sender<HashMap<String, MintTxInfo>>,
        Receiver<HashMap<String, MintTxInfo>>,
    ) = mpsc::channel();

    for (wallet, balance) in holders.list.iter() {
        let wallet_key = Pubkey::from_str(wallet).unwrap();

        let amount_to_mint: u64 = if one_to_wallet { 1 } else { *balance as u64 };

        let thread_url = url.clone();

        let thread_payer = payer.clone();

        let thread_tx = tx.clone();

        tokio::spawn(async move {
            mint(
                mint_key,
                thread_url,
                amount_to_mint,
                wallet_key,
                thread_payer,
                thread_tx,
                sleep,
            )
            .await;
        });
    }

    let mut idx = 0;
    let mut flag = true;

    let mut mint_statuses = MintStatus {
        list: HashMap::new(),
    };

    while flag {
        let tx_status = rx.recv();

        if let Ok(tx) = tx_status {
            mint_statuses.list.extend(tx);

            idx += 1;

            println!("Status of {:?} transaction was saved", idx);
        }

        if idx == holders.list.len() {
            flag = false;
        }
    }

    std::fs::write(
        "./mint_tx_statuses.json",
        serde_json::to_string_pretty(&mint_statuses).unwrap(),
    )
    .unwrap();
}

async fn mint(
    mint: Pubkey,
    url: String,
    amount_to_mint: u64,
    wallet: Pubkey,
    payer: String,
    channel: Sender<HashMap<String, MintTxInfo>>,
    sleep: u64,
) {
    let timeout = Duration::from_secs(10000000);
    let rpc_client = RpcClient::new_with_timeout(&url, timeout);

    let payer = base58_to_keypair(payer.as_ref());

    let associated_token_acc =
        spl_associated_token_account::get_associated_token_address(&wallet, &mint);

    let instructions = vec![
        spl_associated_token_account::create_associated_token_account(
            &payer.pubkey(),
            &wallet,
            &mint,
        ),
        spl_token::instruction::mint_to(
            &spl_token::id(),
            &mint,
            &associated_token_acc,
            &payer.pubkey(),
            &[&payer.pubkey()],
            amount_to_mint,
        )
        .unwrap(),
    ];

    let mut attempts = 5;
    let mut sleep_time = time::Duration::from_millis(sleep);

    while attempts > 0 {
        attempts -= 1;

        let blockhash = get_blockhash(&rpc_client).await;

        if let Some(blockhash) = blockhash {
            let tx = Transaction::new_signed_with_payer(
                &instructions,
                Some(&payer.pubkey()),
                &[&payer],
                blockhash,
            );
    
            let result = rpc_client.send_and_confirm_transaction(&tx);
    
            if let Ok(signature) = result {
                let mut tx_info: HashMap<String, MintTxInfo> = HashMap::new();
    
                tx_info.insert(
                    wallet.to_string(),
                    MintTxInfo {
                        signature: Some(signature.to_string()),
                        status: true,
                    },
                );
    
                channel.send(tx_info).unwrap();
    
                break;
            } else {
                if attempts == 0 {
                    let mut tx_info: HashMap<String, MintTxInfo> = HashMap::new();
    
                    tx_info.insert(
                        wallet.to_string(),
                        MintTxInfo {
                            signature: None,
                            status: false,
                        },
                    );
    
                    channel.send(tx_info).unwrap();
                    break;
                }
    
                thread::sleep(sleep_time);
    
                sleep_time += sleep_time;
            }
        } else {
            thread::sleep(sleep_time);
    
            sleep_time += sleep_time;
        }
    }
}

async fn get_blockhash(client: &RpcClient) -> Option<Hash> {
    let mut attempts = 10;

    while attempts > 0 {
        let blockhash_result = client.get_latest_blockhash();

        if let Ok(hash) = blockhash_result {
            return Some(hash);
        }

        attempts -= 1;

        let sleep_time = time::Duration::from_millis(2000);

        thread::sleep(sleep_time);
    }

    None
}

fn base58_to_keypair(key: &str) -> Keypair {
    let priv_key_bytes = bs58::decode(key).into_vec().unwrap();

    Keypair::from_bytes(priv_key_bytes.as_ref()).unwrap()
}

async fn snapshot(
    output_file: String,
    collection: String,
    url: String,
    time_to_sleep: u64,
    collection_offset: usize,
) {
    let timeout = Duration::from_secs(10000000);
    let rpc_client = RpcClient::new_with_timeout(&url, timeout);

    let memcmp = RpcFilterType::Memcmp(Memcmp {
        offset: collection_offset, // 402
        bytes: MemcmpEncodedBytes::Base64(collection),
        encoding: None,
    });

    let accounts = get_program_accounts(&rpc_client, &mpl_token_metadata::id(), vec![memcmp]);

    println!("LEN: {:?}", accounts.len());

    let (tx, rx): (Sender<Pubkey>, Receiver<Pubkey>) = mpsc::channel();

    for (_, metadata) in accounts.iter().enumerate() {
        let mint_key = Pubkey::new(metadata.1.data.as_ref());

        let thread_tx = tx.clone();

        let thread_url = url.clone();

        tokio::spawn(async move {
            get_nft_holder(mint_key, thread_tx, thread_url, time_to_sleep).await;
        });
    }

    let mut idx = 0;

    let mut holders: HashMap<String, u32> = HashMap::new();

    let mut flag = true;

    while flag {
        let holder = rx.recv();

        if let Ok(holder_key) = holder {
            let count = holders.entry(holder_key.to_string()).or_insert(0);
            *count += 1;

            idx += 1;

            println!("Element number {:?} was set", idx);
        }

        if idx == accounts.len() {
            flag = false;
        }
    }

    let holders_to_write = Holders { list: holders };

    std::fs::write(
        &output_file,
        serde_json::to_string_pretty(&holders_to_write).unwrap(),
    )
    .unwrap();
}

async fn fake_snapshot(amount_of_holders: u64, output_file: String) {
    let mut holders = Holders {
        list: HashMap::new(),
    };

    for _ in 0..amount_of_holders {
        let random_wallet = Keypair::new();

        holders.list.insert(random_wallet.pubkey().to_string(), 1);
    }

    std::fs::write(
        &output_file,
        serde_json::to_string_pretty(&holders).unwrap(),
    )
    .unwrap();
}

#[tokio::main]
async fn main() {
    let args = CliArgs::parse();

    match args.command {
        Commands::MakeSnapshot {
            output_file,
            collection,
            collection_offset,
        } => {
            snapshot(
                output_file,
                collection,
                args.url,
                args.sleep,
                collection_offset,
            )
            .await;
        }
        Commands::MakeFakeSnapshot {
            amount_of_holders,
            output_file,
        } => {
            fake_snapshot(amount_of_holders, output_file).await;
        }
        Commands::Airdrop {
            mint,
            holders_list,
            one_to_wallet,
        } => {
            let mut file = File::open(holders_list).unwrap();
            let mut data = String::new();
            file.read_to_string(&mut data).unwrap();

            let holders: Holders = serde_json::from_str(&data).unwrap();

            airdrop(
                Pubkey::from_str(&mint).unwrap(),
                args.url,
                holders,
                args.payer_keypair,
                one_to_wallet,
                args.sleep,
            )
            .await;
        }
    }
}
