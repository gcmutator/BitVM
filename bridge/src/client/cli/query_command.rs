use alloy::primitives::Address;
use bitcoin::{Amount, Network, OutPoint, PublicKey, XOnlyPublicKey};
use clap::{arg, ArgMatches, Command};
use core::str::FromStr;

use super::{
    query_response::{Response, ResponseStatus},
    validation::{validate, ArgType},
};
use crate::{
    client::{
        chain::chain_adaptor::get_chain_adaptor,
        client::BitVMClient,
        esplora::get_esplora_url,
        sdk::{query::ClientCliQuery, query_contexts::depositor_signatures::DepositorSignatures},
    },
    constants::DestinationNetwork,
    contexts::base::generate_keys_from_secret,
    scripts::generate_pay_to_pubkey_script_address,
    transactions::base::Input,
};

pub struct QueryCommand {
    client: BitVMClient,
    network: Network,
}

const VERIFIER_0_SECRET: &str = "ee0817eac0c13aa8ee2dd3256304041f09f0499d1089b56495310ae8093583e2";
const FAKE_SECRET: &str = "1000000000000000000000000000000000000000000000000000000000000000";

const QUERY_COMMAND_PATH_PREFIX: &str = "query_command";

impl QueryCommand {
    pub async fn new(
        source_network: Network,
        destination_network: DestinationNetwork,
        path_prefix: Option<&str>,
    ) -> Self {
        let (_, verifier_0_public_key) =
            generate_keys_from_secret(source_network, VERIFIER_0_SECRET);

        let n_of_n_public_keys: Vec<PublicKey> = vec![verifier_0_public_key];

        let bitvm_client = BitVMClient::new(
            Some(get_esplora_url(source_network)),
            source_network,
            destination_network,
            Some(get_chain_adaptor(DestinationNetwork::Local, None, None)), // TODO: Update this according to the requirements for query command.
            &n_of_n_public_keys,
            Some(FAKE_SECRET),
            Some(FAKE_SECRET),
            Some(VERIFIER_0_SECRET),
            Some(FAKE_SECRET),
            path_prefix.or(Some(QUERY_COMMAND_PATH_PREFIX)),
            None,
        )
        .await;

        Self {
            client: bitvm_client,
            network: source_network,
        }
    }

    async fn sync(&mut self) {
        self.client.sync().await;
        self.client.sync_l2().await;
    }

    pub fn pegin_deposit_tx_command() -> Command {
        Command::new("pegin_deposit_tx")
            .about("Subcommand for handling pegin deposit transactions")
            .arg(arg!(<AMOUNT> "Amount of assets to peg-in").required(true))
            .arg(
                arg!(<RECIPIENT_ADDRESS> "Recipient L2 chain address for peg-in transaction")
                    .required(true),
            )
    }

    pub fn pegin_confirm_tx_command() -> Command {
        Command::new("pegin_confirm_tx")
            .about("Subcommand for handling pegin confirm transactions")
            .arg(arg!(<AMOUNT> "Amount of assets to peg-in").required(true))
            .arg(
                arg!(<RECIPIENT_ADDRESS> "Recipient L2 chain address for peg-in transaction")
                    .required(true),
            )
    }

    pub fn pegin_refund_tx_command() -> Command {
        Command::new("pegin_refund_tx")
            .about("Subcommand for handling pegin refund transactions")
            .arg(arg!(<AMOUNT> "Amount of assets to peg-in").required(true))
            .arg(
                arg!(<RECIPIENT_ADDRESS> "Recipient L2 chain address for peg-in transaction")
                    .required(true),
            )
    }

    pub async fn handle_pegin_refund_tx_command(
        &self,
        depositor_public_key: &PublicKey,
        sub_matches: &ArgMatches,
    ) -> Response {
        let amount = sub_matches.get_one::<String>("AMOUNT").unwrap();
        let recipient_address = sub_matches.get_one::<String>("RECIPIENT_ADDRESS").unwrap();
        let depositor_taproot_key = XOnlyPublicKey::from(*depositor_public_key);
        let amount: Amount = Amount::from_sat(amount.parse::<u64>().unwrap());
        let taproot_address = self.client.generate_connector_z_taproot_address(
            self.network,
            recipient_address,
            &depositor_taproot_key,
        );
        let outpoint = self
            .generate_stub_outpoint(&self.client, &taproot_address, amount)
            .await;
        let result = self.client.generate_presign_pegin_refund_tx(
            self.network,
            amount,
            recipient_address,
            depositor_public_key,
            outpoint,
        );
        Response::new(
            ResponseStatus::OK,
            Some(serde_json::to_value(result).unwrap()),
        )
    }

    pub async fn handle_pegin_deposit_tx_command(
        &self,
        depositor_public_key: &PublicKey,
        sub_matches: &ArgMatches,
    ) -> Response {
        let amount = sub_matches.get_one::<String>("AMOUNT").unwrap();
        let recipient_address = sub_matches.get_one::<String>("RECIPIENT_ADDRESS").unwrap();
        let amount: Amount = Amount::from_sat(amount.parse::<u64>().unwrap());
        let outpoint = self
            .generate_stub_outpoint(
                &self.client,
                &generate_pay_to_pubkey_script_address(self.network, depositor_public_key),
                amount,
            )
            .await;
        let result = self.client.generate_presign_pegin_deposit_tx(
            self.network,
            amount,
            recipient_address,
            depositor_public_key,
            outpoint,
        );
        Response::new(
            ResponseStatus::OK,
            Some(serde_json::to_value(result).unwrap()),
        )
    }

    pub fn depositor_command() -> Command {
        Command::new("depositor")
            .about("fetch peg-in graphs related to the specified depositor")
            .arg(arg!(<DEPOSITOR_PUBLIC_KEY> "Depositor public key").required(true))
            .subcommand(Self::pegin_deposit_tx_command())
            .subcommand(Self::pegin_confirm_tx_command())
            .subcommand(Self::pegin_refund_tx_command())
    }

    pub async fn handle_pegin_confirm_tx_command(
        &self,
        depositor_public_key: &PublicKey,
        sub_matches: &ArgMatches,
    ) -> Response {
        let amount = sub_matches.get_one::<String>("AMOUNT").unwrap();
        let recipient_address = sub_matches.get_one::<String>("RECIPIENT_ADDRESS").unwrap();
        let depositor_taproot_key = XOnlyPublicKey::from(*depositor_public_key);
        let amount: Amount = Amount::from_sat(amount.parse::<u64>().unwrap());
        let taproot_address = self.client.generate_connector_z_taproot_address(
            self.network,
            recipient_address,
            &depositor_taproot_key,
        );
        let outpoint = self
            .generate_stub_outpoint(&self.client, &taproot_address, amount)
            .await;
        let result = self.client.generate_presign_pegin_confirm_tx(
            self.network,
            recipient_address,
            amount,
            &depositor_taproot_key,
            outpoint,
        );
        Response::new(
            ResponseStatus::OK,
            Some(serde_json::to_value(result).unwrap()),
        )
    }

    pub async fn handle_depositor(&mut self, matches: &ArgMatches) -> Response {
        let pubkey =
            PublicKey::from_str(matches.get_one::<String>("DEPOSITOR_PUBLIC_KEY").unwrap());
        if pubkey.is_err() {
            return Response::new(
                ResponseStatus::NOK(
                    "Invalid public key. Use bitcoin public key format.".to_string(),
                ),
                None,
            );
        }
        if matches.subcommand_matches("pegin_deposit_tx").is_some() {
            return self
                .handle_pegin_deposit_tx_command(
                    &pubkey.unwrap(),
                    matches.subcommand_matches("pegin_deposit_tx").unwrap(),
                )
                .await;
        }
        if matches.subcommand_matches("pegin_confirm_tx").is_some() {
            return self
                .handle_pegin_confirm_tx_command(
                    &pubkey.unwrap(),
                    matches.subcommand_matches("pegin_confirm_tx").unwrap(),
                )
                .await;
        }
        if matches.subcommand_matches("pegin_refund_tx").is_some() {
            return self
                .handle_pegin_refund_tx_command(
                    &pubkey.unwrap(),
                    matches.subcommand_matches("pegin_refund_tx").unwrap(),
                )
                .await;
        }

        self.sync().await;
        let result = self
            .client
            .get_depositor_status(&pubkey.clone().unwrap())
            .await;

        match result.len() {
            len if len > 0 => {
                let data =
                    Some(serde_json::to_value(result).expect("Failed to merge value vector"));
                Response::new(ResponseStatus::OK, data)
            }
            _ => Response::new(
                ResponseStatus::NOK("Depositor not found.".to_string()),
                None,
            ),
        }
    }

    pub fn withdrawer_command() -> Command {
        Command::new("withdrawer")
            .about("fetch peg-out graphs related to the specified withdrawer")
            .arg(arg!(<WITHDRAWER_CHAIN_ADDRESS> "WITHDRAWER L2 Chain address").required(true))
    }

    pub async fn handle_withdrawer(
        &mut self,
        matches: &ArgMatches,
        destination_network: DestinationNetwork,
    ) -> Response {
        let chain_address = Address::from_str(
            matches
                .get_one::<String>("WITHDRAWER_CHAIN_ADDRESS")
                .unwrap(),
        );
        if chain_address.is_err() {
            return Response::new(
                ResponseStatus::NOK(format!(
                    "Invalid chain address. Use {} address format.",
                    destination_network
                )),
                None,
            );
        }

        self.sync().await;
        let result = self
            .client
            .get_withdrawer_status(chain_address.unwrap().to_string().as_str())
            .await;

        match result.len() {
            len if len > 0 => {
                let data =
                    Some(serde_json::to_value(result).expect("Failed to merge value vector"));
                Response::new(ResponseStatus::OK, data)
            }
            _ => Response::new(
                ResponseStatus::NOK("Withdrawer not found.".to_string()),
                None,
            ),
        }
    }

    pub fn history_command() -> Command {
        Command::new("history")
            .about("fetch peg-in / peg-out graphs with bitcoin public key and ethereum address at the same time")
            .arg(arg!(<DEPOSITOR_PUBLIC_KEY> "Depositor public key").required(true))
            .arg(arg!(<WITHDRAWER_CHAIN_ADDRESS> "WITHDRAWER L2 Chain address").required(true))
    }

    pub async fn handle_history(
        &mut self,
        matches: &ArgMatches,
        destination_network: DestinationNetwork,
    ) -> Response {
        let args = vec![
            "DEPOSITOR_PUBLIC_KEY".to_string(),
            "WITHDRAWER_CHAIN_ADDRESS".into(),
        ];
        let validate_result = match validate(matches, args, destination_network) {
            Ok(result) => result,
            Err(err) => return err,
        };
        let (pubkey, chain_address) = match &validate_result[..] {
            [arg1, arg2, ..] => match (arg1, arg2) {
                (ArgType::DepositorPublicKey(pubkey), ArgType::ChainAddress(chain_address)) => {
                    (pubkey, chain_address)
                }
                _ => unreachable!(),
            },
            _ => unreachable!(),
        };

        self.sync().await;
        let mut result_depositor = self.client.get_depositor_status(pubkey).await;
        let mut result_withdrawer = self
            .client
            .get_withdrawer_status(chain_address.to_string().as_str())
            .await;

        let result = match (result_depositor.len(), result_withdrawer.len()) {
            (0, 0) => vec![],
            (0, _) => result_withdrawer,
            (_, 0) => result_depositor,
            _ => {
                result_depositor.append(&mut result_withdrawer);
                result_depositor
            }
        };

        match result.len() {
            len if len > 0 => {
                let data =
                    Some(serde_json::to_value(result).expect("Failed to merge value vector"));
                Response::new(ResponseStatus::OK, data)
            }
            _ => Response::new(
                ResponseStatus::NOK("Depositor / Withdrawer not found.".to_string()),
                None,
            ),
        }
    }

    pub fn transactions_command() -> Command {
        Command::new("transactions")
            .about("create transactions of peg-in graph for depositor to sign")
            .arg(arg!(<DEPOSITOR_PUBLIC_KEY> "Depositor public key").required(true))
            .arg(arg!(<DESTINATION_CHAIN_ADDRESS> "Depositor's destination address on L2 Chain").required(true))
            .arg(arg!(<OUTPOINT> "Previous output for peg-in deposit transaction input, format: <txid>:<vout>").required(true))
            .arg(arg!(<SATS> "Amount of satoshis to deposit, should be also the value of previous output").required(true))
    }

    pub async fn handle_transactions(
        &self,
        matches: &ArgMatches,
        destination_network: DestinationNetwork,
    ) -> Response {
        let args = vec![
            "DEPOSITOR_PUBLIC_KEY".to_string(),
            "DESTINATION_CHAIN_ADDRESS".into(),
            "OUTPOINT".into(),
            "SATS".into(),
        ];
        let validate_result = match validate(matches, args, destination_network) {
            Ok(result) => result,
            Err(err) => return err,
        };
        let (pubkey, chain_address, outpoint, satoshis) = match &validate_result[..] {
            [arg1, arg2, arg3, arg4, ..] => match (arg1, arg2, arg3, arg4) {
                (
                    ArgType::DepositorPublicKey(pubkey),
                    ArgType::ChainAddress(chain_address),
                    ArgType::OutPoint(outpoint),
                    ArgType::Satoshis(satoshis),
                ) => (pubkey, chain_address, outpoint, satoshis),
                _ => unreachable!(),
            },
            _ => unreachable!(),
        };
        let x_only_pubkey = XOnlyPublicKey::from(*pubkey);

        // do not need to sync
        let result = self
            .client
            .get_depositor_transactions(
                &pubkey.clone(),
                &x_only_pubkey,
                Input {
                    outpoint: *outpoint,
                    amount: *satoshis,
                },
                chain_address.to_string().as_str(),
            )
            .await;

        match result {
            Ok(result) => Response::new(ResponseStatus::OK, Some(result)),
            Err(err) => Response::new(ResponseStatus::NOK(err), None),
        }
    }

    pub fn signatures_command() -> Command {
        Command::new("signatures")
            .about("create peg-in graph and broadcast peg-in deposit with depositor signatures")
            .arg(arg!(<DEPOSITOR_PUBLIC_KEY> "Depositor public key").required(true))
            .arg(arg!(<DESTINATION_CHAIN_ADDRESS> "Depositor's destination address on L2 Chain").required(true))
            .arg(arg!(<OUTPOINT> "Previous output for peg-in deposit transaction input, format: <txid>:<vout>").required(true))
            .arg(arg!(<SATS> "Amount of satoshis to deposit, should be also the value of previous output").required(true))
            .arg(arg!(<DEPOSIT> "Signature hex for peg-in deposit").required(true))
            .arg(arg!(<CONFIRM> "Signature hex for peg-in confirm").required(true))
            .arg(arg!(<REFUND> "Signature hex for peg-in refund").required(true))
    }

    pub async fn handle_signatures(
        &mut self,
        matches: &ArgMatches,
        destination_network: DestinationNetwork,
    ) -> Response {
        let args = vec![
            "DEPOSITOR_PUBLIC_KEY".to_string(),
            "DESTINATION_CHAIN_ADDRESS".into(),
            "OUTPOINT".into(),
            "SATS".into(),
            "DEPOSIT".into(),
            "CONFIRM".into(),
            "REFUND".into(),
        ];
        let validate_result = match validate(matches, args, destination_network) {
            Ok(result) => result,
            Err(err) => return err,
        };
        let (pubkey, chain_address, outpoint, satoshis, deposit, confirm, refund) =
            match &validate_result[..] {
                [arg1, arg2, arg3, arg4, arg5, arg6, arg7, ..] => {
                    match (arg1, arg2, arg3, arg4, arg5, arg6, arg7) {
                        (
                            ArgType::DepositorPublicKey(pubkey),
                            ArgType::ChainAddress(chain_address),
                            ArgType::OutPoint(outpoint),
                            ArgType::Satoshis(satoshis),
                            ArgType::EcdsaSignature(deposit),
                            ArgType::TaprootSignature(confirm),
                            ArgType::TaprootSignature(refund),
                        ) => (
                            pubkey,
                            chain_address,
                            outpoint,
                            satoshis,
                            deposit,
                            confirm,
                            refund,
                        ),
                        _ => unreachable!(),
                    }
                }
                _ => unreachable!(),
            };
        let x_only_pubkey = XOnlyPublicKey::from(*pubkey);

        self.sync().await;
        let result = self
            .client
            .create_peg_in_graph_with_depositor_signatures(
                pubkey,
                &x_only_pubkey,
                Input {
                    outpoint: *outpoint,
                    amount: *satoshis,
                },
                chain_address.to_string().as_str(),
                &DepositorSignatures {
                    deposit: *deposit,
                    refund: *refund,
                    confirm: *confirm,
                },
            )
            .await;

        match result {
            Ok(result) => Response::new(ResponseStatus::OK, Some(result)),
            Err(err) => Response::new(ResponseStatus::NOK(err), None),
        }
    }

    pub fn broadcast_command() -> Command {
        Command::new("broadcast")
            .about("broadcast peg-in deposit transaction in a peg-in graph separately")
            .args([arg!(<GRAPH_ID> "peg-in graph id").required(true)])
    }

    pub async fn handle_broadcast(&mut self, matches: &ArgMatches) -> Response {
        self.sync().await;
        let arg = "GRAPH_ID";
        let Some(peg_in_graph_id) = matches.get_one::<String>(arg) else {
            return Response::new(
                ResponseStatus::NOK(format!("Missing argument: {}", arg)),
                None,
            );
        };
        let result = self
            .client
            .retry_broadcast_peg_in_deposit(peg_in_graph_id)
            .await;

        match result {
            Ok(result) => Response::new(ResponseStatus::OK, Some(result)),
            Err(err) => Response::new(ResponseStatus::NOK(err), None),
        }
    }

    pub fn peg_in_graphs_command() -> Command {
        Command::new("pegins")
            .about("fetch all yet available peg-in graphs for pegging out process")
    }

    pub async fn handle_peg_in_graphs(&mut self) -> Response {
        self.sync().await;
        let result = self.client.get_unused_peg_in_graphs().await;

        match result.len() {
            len if len > 0 => {
                let data =
                    Some(serde_json::to_value(result).expect("Failed to merge value vector"));
                Response::new(ResponseStatus::OK, data)
            }
            _ => Response::new(
                ResponseStatus::NOK("No available peg-in graphs found.".to_string()),
                None,
            ),
        }
    }

    pub async fn generate_stub_outpoint(
        &self,
        client: &BitVMClient,
        funding_utxo_address: &bitcoin::Address,
        input_value: Amount,
    ) -> OutPoint {
        let funding_utxo = client
            .get_initial_utxo(funding_utxo_address.clone(), input_value)
            .await
            .unwrap_or_else(|| {
                panic!(
                    "Fund {:?} with {} sats at {}",
                    funding_utxo_address,
                    input_value.to_sat(),
                    client.esplora.url(),
                );
            });
        OutPoint {
            txid: funding_utxo.txid,
            vout: funding_utxo.vout,
        }
    }
}
