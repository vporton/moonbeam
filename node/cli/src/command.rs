// Copyright 2019-2021 PureStake Inc.
// This file is part of Moonbeam.

// Moonbeam is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Moonbeam is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Moonbeam.  If not, see <http://www.gnu.org/licenses/>.

//! This module constructs and executes the appropriate service components for the given subcommand

use crate::cli::{Cli, RelayChainCli, Subcommand};
use cli_opt::RpcParams;
use cumulus_client_service::genesis::generate_genesis_block;
use cumulus_primitives_core::ParaId;
use log::info;
// TODO-multiple-runtimes
#[cfg(feature = "with-moonbase-runtime")]
use service::moonbase_runtime::{AccountId, Block};
#[cfg(feature = "with-moonbeam-runtime")]
use service::moonbeam_runtime::{AccountId, Block};
use parity_scale_codec::Encode;
use polkadot_parachain::primitives::AccountIdConversion;
use polkadot_service::RococoChainSpec;
use sc_cli::{
	ChainSpec, CliConfiguration, DefaultConfigurationValues, ImportParams, KeystoreParams,
	NetworkParams, Result, RuntimeVersion, SharedParams, SubstrateCli,
};
use sc_service::{
	config::{BasePath, PrometheusConfig},
	PartialComponents,
};
use sp_core::hexdisplay::HexDisplay;
use sp_core::H160;
use sp_runtime::traits::Block as _;
use std::{io::Write, net::SocketAddr, str::FromStr};
use service::{chain_spec, IdentifyVariant};

fn load_spec(
	id: &str,
	para_id: ParaId,
) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
	if id.is_empty() {
		return Err("Not specific which chain to run.".into());
	}
	Ok(match id {
		#[cfg(feature = "with-moonbase-runtime")]
		"local" => Box::new(chain_spec::moonbase::get_chain_spec(para_id)),
		#[cfg(feature = "with-moonbase-runtime")]
		"dev" | "development" => Box::new(chain_spec::moonbase::development_chain_spec(None, None)),
		#[cfg(feature = "with-moonbase-runtime")]
		"alphanet" => Box::new(chain_spec::moonbase::ChainSpec::from_json_bytes(
			&include_bytes!("../../../specs/alphanet/parachain-embedded-specs-v7.json")[..],
		)?),
		#[cfg(feature = "with-moonbase-runtime")]
		"stagenet" => Box::new(chain_spec::moonbase::ChainSpec::from_json_bytes(
			&include_bytes!("../../../specs/stagenet/parachain-embedded-specs-v7.json")[..],
		)?),
		// TODO-multiple-runtimes test-spec staking
		// TODO-multiple-runtimes live release
		#[cfg(feature = "with-moonbeam-runtime")]
		"moonbeam" => Box::new(chain_spec::moonbeam::development_chain_spec(None, None)),
		// TODO-multiple-runtimes
		path => {
			#[cfg(feature = "with-moonbeam-runtime")]
			{
				Box::new(chain_spec::moonbeam::ChainSpec::from_json_file(
					path.into(),
				)?)
			}
			
			#[cfg(feature = "with-moonbase-runtime")]
			{
				Box::new(chain_spec::moonbase::ChainSpec::from_json_file(
					path.into(),
				)?)
			}
		},
	})
}

impl SubstrateCli for Cli {
	fn impl_name() -> String {
		"Moonbase Parachain Collator".into()
	}

	fn impl_version() -> String {
		env!("SUBSTRATE_CLI_IMPL_VERSION").into()
	}

	fn description() -> String {
		format!(
			"Moonbase Parachain Collator\n\nThe command-line arguments provided first will be \
		passed to the parachain node, while the arguments provided after -- will be passed \
		to the relaychain node.\n\n\
		{} [parachain-args] -- [relaychain-args]",
			Self::executable_name()
		)
	}

	fn author() -> String {
		env!("CARGO_PKG_AUTHORS").into()
	}

	fn support_url() -> String {
		"https://github.com/PureStake/moonbeam/issues/new".into()
	}

	fn copyright_start_year() -> i32 {
		2019
	}

	fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
		load_spec(id, self.run.parachain_id.unwrap_or(1000).into())
	}

	fn native_runtime_version(spec: &Box<dyn sc_service::ChainSpec>) -> &'static RuntimeVersion {
		if spec.is_moonbase() {
			#[cfg(feature = "with-moonbase-runtime")]
			return &service::moonbase_runtime::VERSION;
			#[cfg(not(feature = "with-moonbase-runtime"))]
			panic!("Moonbase runtime is not available. Please compile the node with `--features with-moonbase-runtime` to enable it.");
		} else {
			#[cfg(feature = "with-moonbeam-runtime")]
			return &service::moonbeam_runtime::VERSION;
			#[cfg(not(feature = "with-moonbeam-runtime"))]
			panic!("Moonbeam runtime is not available. Please compile the node with `--features with-moonbeam-runtime` to enable it.");
		}
	}
}

impl SubstrateCli for RelayChainCli {
	fn impl_name() -> String {
		"Moonbeam Parachain Collator".into()
	}

	fn impl_version() -> String {
		env!("SUBSTRATE_CLI_IMPL_VERSION").into()
	}

	fn description() -> String {
		"Moonbeam Parachain Collator\n\nThe command-line arguments provided first will be \
		passed to the parachain node, while the arguments provided after -- will be passed \
		to the relaychain node.\n\n\
		parachain-collator [parachain-args] -- [relaychain-args]"
			.into()
	}

	fn author() -> String {
		env!("CARGO_PKG_AUTHORS").into()
	}

	fn support_url() -> String {
		"https://github.com/PureStake/moonbeam/issues/new".into()
	}

	fn copyright_start_year() -> i32 {
		2019
	}

	fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
		match id {
			"moonbase_alpha_relay" => Ok(Box::new(RococoChainSpec::from_json_bytes(
				&include_bytes!("../../../specs/alphanet/rococo-embedded-specs-v7.json")[..],
			)?)),
			"moonbase_stage_relay" => Ok(Box::new(RococoChainSpec::from_json_bytes(
				&include_bytes!("../../../specs/stagenet/rococo-embedded-specs-v7.json")[..],
			)?)),
			// If we are not using a moonbeam-centric pre-baked relay spec, then fall back to the
			// Polkadot service to interpret the id.
			_ => polkadot_cli::Cli::from_iter([RelayChainCli::executable_name()].iter())
				.load_spec(id),
		}
	}

	fn native_runtime_version(chain_spec: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
		polkadot_cli::Cli::native_runtime_version(chain_spec)
	}
}

#[allow(clippy::borrowed_box)]
fn extract_genesis_wasm(chain_spec: &Box<dyn sc_service::ChainSpec>) -> Result<Vec<u8>> {
	let mut storage = chain_spec.build_storage()?;

	storage
		.top
		.remove(sp_core::storage::well_known_keys::CODE)
		.ok_or_else(|| "Could not find wasm file in genesis state!".into())
}

/// Parse command line arguments into service configuration.
pub fn run() -> Result<()> {
	let cli = Cli::from_args();
	let author_id: Option<H160> = cli.run.author_id;
	match &cli.subcommand {
		Some(Subcommand::BuildSpec(params)) => {
			let runner = cli.create_runner(&params.base)?;
			runner.sync_run(|config| {
				if params.mnemonic.is_some() || params.accounts.is_some() {
					if config.chain_spec.is_moonbeam() {
						// TODO-multiple-runtimes
						#[cfg(feature = "with-moonbeam-runtime")]
						{
							params.base.run(
								Box::new(chain_spec::moonbeam::development_chain_spec(
									params.mnemonic.clone(),
									params.accounts,
								)),
								config.network,
							)

						}
						#[cfg(not(feature = "with-moonbeam-runtime"))]
						return Err("Moonbeam runtime is not available. Please compile the node with `--features with-moonbeam-runtime` to enable it.".into());
					} else {
						#[cfg(feature = "with-moonbase-runtime")]
						{
							params.base.run(
								Box::new(chain_spec::moonbase::development_chain_spec(
									params.mnemonic.clone(),
									params.accounts,
								)),
								config.network,
							)
						}
						#[cfg(not(feature = "with-moonbase-runtime"))]
						return Err("Moonbase runtime is not available. Please compile the node with `--features with-moonbase-runtime` to enable it.".into());
					}
				} else {
					params.base.run(config.chain_spec, config.network)
				}
			})
		}
		Some(Subcommand::CheckBlock(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			runner.async_run(|mut config| {
				let (client, _, import_queue, task_manager) = service::new_chain_ops(&mut config, author_id)?;
				Ok((cmd.run(client, import_queue), task_manager))
			})
		}
		Some(Subcommand::ExportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			runner.async_run(|mut config| {
				let (client, _, _, task_manager) = service::new_chain_ops(&mut config, author_id)?;
				Ok((cmd.run(client, config.database), task_manager))
			})
		}
		Some(Subcommand::ExportState(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			runner.async_run(|mut config| {
				let (client, _, _, task_manager) = service::new_chain_ops(&mut config, author_id)?;
				Ok((cmd.run(client, config.chain_spec), task_manager))
			})
		}
		Some(Subcommand::ImportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			runner.async_run(|mut config| {
				let (client, _, import_queue, task_manager) = service::new_chain_ops(&mut config, author_id)?;
				Ok((cmd.run(client, import_queue), task_manager))
			})
		}
		Some(Subcommand::PurgeChain(cmd)) => {
			// TODO-multiple-runtimes
			let runner = cli.create_runner(cmd)?;

			runner.sync_run(|config| {
				// Although the cumulus_client_cli::PurgeCommand will extract the relay chain id,
				// we need to extract it here to determine whether we are running the dev service.
				let extension = {
					#[cfg(feature = "with-moonbase-runtime")]
					{
						chain_spec::moonbase::Extensions::try_get(&*config.chain_spec)
					}
					#[cfg(feature = "with-moonbeam-runtime")]
					{
						chain_spec::moonbeam::Extensions::try_get(&*config.chain_spec)
					}
				};
				let relay_chain_id = extension.map(|e| e.relay_chain.clone());
				let dev_service =
					cli.run.dev_service || relay_chain_id == Some("dev-service".to_string());

				if dev_service {
					// base refers to the encapsulated "regular" sc_cli::PurgeChain command
					return cmd.base.run(config.database);
				}

				let polkadot_cli = RelayChainCli::new(
					&config,
					[RelayChainCli::executable_name().to_string()]
						.iter()
						.chain(cli.relaychain_args.iter()),
				);

				let polkadot_config = SubstrateCli::create_configuration(
					&polkadot_cli,
					&polkadot_cli,
					config.task_executor.clone(),
				)
				.map_err(|err| format!("Relay chain argument error: {}", err))?;

				cmd.run(config, polkadot_config)
			})
		}
		Some(Subcommand::Revert(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			runner.async_run(|mut config| {
				let (client, backend, _, task_manager) = service::new_chain_ops(&mut config, author_id)?;
				Ok((cmd.run(client, backend), task_manager))
			})
		}
		Some(Subcommand::ExportGenesisState(params)) => {
			let mut builder = sc_cli::LoggerBuilder::new("");
			builder.with_profiling(sc_tracing::TracingReceiver::Log, "");
			let _ = builder.init();

			let chain_spec = cli.load_spec(&params.chain.clone().unwrap_or_default())?;
			let output_buf = if chain_spec.is_moonbeam() {
				#[cfg(feature = "with-moonbeam-runtime")]
				{
					let block: service::moonbeam_runtime::Block =
						generate_genesis_block(&chain_spec).map_err(|e| format!("{:?}", e))?;
					let raw_header = block.header().encode();
					let output_buf = if params.raw {
						raw_header
					} else {
						format!("0x{:?}", HexDisplay::from(&block.header().encode())).into_bytes()
					};
					output_buf
				}
				#[cfg(not(feature = "with-moonbeam-runtime"))]
				return Err("Moonbeam runtime is not available. Please compile the node with `--features with-moonbeam-runtime` to enable it.".into());
			} else {
				#[cfg(feature = "with-moonbase-runtime")]
				{
					let block: service::moonbase_runtime::Block = generate_genesis_block(&chain_spec)?;
					let raw_header = block.header().encode();
					let output_buf = if params.raw {
						raw_header
					} else {
						format!("0x{:?}", HexDisplay::from(&block.header().encode())).into_bytes()
					};
					output_buf
				}
				#[cfg(not(feature = "with-moonbase-runtime"))]
				return Err("Moonbase runtime is not available. Please compile the node with `--features with-moonbase-runtime` to enable it.".into());
			};

			if let Some(output) = &params.output {
				std::fs::write(output, output_buf)?;
			} else {
				std::io::stdout().write_all(&output_buf)?;
			}

			Ok(())
		}
		Some(Subcommand::ExportGenesisWasm(params)) => {
			// TODO-multiple-runtimes?
			let mut builder = sc_cli::LoggerBuilder::new("");
			builder.with_profiling(sc_tracing::TracingReceiver::Log, "");
			let _ = builder.init();

			let raw_wasm_blob =
				extract_genesis_wasm(&cli.load_spec(&params.chain.clone().unwrap_or_default())?)?;
			let output_buf = if params.raw {
				raw_wasm_blob
			} else {
				format!("0x{:?}", HexDisplay::from(&raw_wasm_blob)).into_bytes()
			};

			if let Some(output) = &params.output {
				std::fs::write(output, output_buf)?;
			} else {
				std::io::stdout().write_all(&output_buf)?;
			}

			Ok(())
		}
		Some(Subcommand::Benchmark(cmd)) => {
			if cfg!(feature = "runtime-benchmarks") {
				let runner = cli.create_runner(cmd)?;
				let chain_spec = &runner.config().chain_spec;
				if chain_spec.is_moonbeam() {
					#[cfg(feature = "with-moonbeam-runtime")]
					return runner.sync_run(|config| cmd.run::<service::moonbeam_runtime::Block, service::MoonbeamExecutor>(config));
					#[cfg(not(feature = "with-moonbeam-runtime"))]
					return Err("Moonbeam runtime is not available. Please compile the node with `--features with-moonbeam-runtime` to enable it.".into());
				} else {
					#[cfg(feature = "with-moonbase-runtime")]
					return runner.sync_run(|config| cmd.run::<service::moonbase_runtime::Block, service::MoonbaseExecutor>(config));
					#[cfg(not(feature = "with-moonbase-runtime"))]
					return Err("Moonbase runtime is not available. Please compile the node with `--features with-moonbase-runtime` to enable it.".into());
				}
			} else {
				Err("Benchmarking wasn't enabled when building the node. \
				You can enable it with `--features runtime-benchmarks`."
					.into())
			}
		}
		None => {
			let runner = cli.create_runner(&*cli.run)?;
			runner
				.run_node_until_exit(|config| async move {

					let collator = cli.run.base.validator || cli.collator;
					let author_id: Option<H160> = cli.run.author_id;
					if collator && author_id.is_none() {
						return Err("Collator nodes must specify an author account id".into());
					}

					let key = sp_core::Pair::generate().0;

					let extension = {
						#[cfg(feature = "with-moonbase-runtime")]
						{
							chain_spec::moonbase::Extensions::try_get(&*config.chain_spec)
						}
						#[cfg(feature = "with-moonbeam-runtime")]
						{
							chain_spec::moonbeam::Extensions::try_get(&*config.chain_spec)
						}
					};
					let relay_chain_id = extension.map(|e| e.relay_chain.clone());
					let dev_service =
						config.chain_spec.is_moonbase_dev() || relay_chain_id == Some("dev-service".to_string());
					let para_id = extension.map(|e| e.para_id);

					let rpc_params = RpcParams {
						ethapi_max_permits: cli.run.ethapi_max_permits,
						ethapi_trace_max_count: cli.run.ethapi_trace_max_count,
						ethapi_trace_cache_duration: cli.run.ethapi_trace_cache_duration,
						max_past_logs: cli.run.max_past_logs,
					};

					// If dev service was requested, start up manual or instant seal.
					// Otherwise continue with the normal parachain node.
					// Dev service can be requested in two ways.
					// 1. by providing the --dev-service flag to the CLI
					// 2. by specifying "dev-service" in the chain spec's "relay-chain" field.
					// NOTE: the --dev flag triggers the dev service by way of number 2
					if dev_service {
						// --dev implies --collator
						let collator = collator || cli.run.shared_params.dev;

						// If no author id was supplied, use the one that is staked at genesis
						// in the default development spec.
						let author_id = author_id.or_else(|| {
							Some(
								AccountId::from_str("6Be02d1d3665660d22FF9624b7BE0551ee1Ac91b")
									.expect("Gerald is a valid account"),
							)
						});
						#[cfg(feature = "with-moonbase-runtime")]
						return service::new_dev(config, author_id, collator, cli.run.sealing, cli.run.ethapi, rpc_params).map_err(Into::into);
						#[cfg(not(feature = "with-moonbase-runtime"))]
						return Err("Moonbase runtime is not available. Please compile the node with `--features with-moonbase-runtime` to enable it.".into());
					}

					let polkadot_cli = RelayChainCli::new(
						&config,
						[RelayChainCli::executable_name().to_string()]
							.iter()
							.chain(cli.relaychain_args.iter()),
					);

					let id = ParaId::from(cli.run.parachain_id.or(para_id).unwrap_or(1000));

					let parachain_account =
						AccountIdConversion::<polkadot_primitives::v0::AccountId>::into_account(
							&id,
						);
					
					let genesis_state = if config.chain_spec.is_moonbeam() {
						#[cfg(feature = "with-moonbeam-runtime")]
						{
							let block: service::moonbeam_runtime::Block = generate_genesis_block(&config.chain_spec).map_err(|e| format!("{:?}", e))?;
							format!("0x{:?}", HexDisplay::from(&block.header().encode()))
						}
						#[cfg(not(feature = "with-moonbeam-runtime"))]
						return Err("Moonbeam runtime is not available. Please compile the node with `--features with-moonbeam-runtime` to enable it.".into());
					} else {
						#[cfg(feature = "with-moonbase-runtime")]
						{
							let block: service::moonbase_runtime::Block = generate_genesis_block(&config.chain_spec).map_err(|e| format!("{:?}", e))?;
							format!("0x{:?}", HexDisplay::from(&block.header().encode()))
						}
						#[cfg(not(feature = "with-moonbase-runtime"))]
						return Err("Moonbase runtime is not available. Please compile the node with `--features with-moonbase-runtime` to enable it.".into());
					};

					let task_executor = config.task_executor.clone();
					let polkadot_config = SubstrateCli::create_configuration(
						&polkadot_cli,
						&polkadot_cli,
						task_executor,
					)
					.map_err(|err| format!("Relay chain argument error: {}", err))?;

					info!("Parachain id: {:?}", id);
					info!("Parachain Account: {}", parachain_account);
					info!("Parachain genesis state: {}", genesis_state);
					info!("Is collating: {}", if collator { "yes" } else { "no" });

					if config.chain_spec.is_moonbeam() {
						#[cfg(feature = "with-moonbeam-runtime")]
						{
							service::start_node::<service::moonbeam_runtime::RuntimeApi, service::MoonbeamExecutor>(
								config,
								key,
								author_id,
								polkadot_config,
								id,
								collator,
								cli.run.sealing,
								cli.run.ethapi,
								rpc_params,
							)
							.await
							.map(|r| r.0)
							.map_err(Into::into)
						}
						#[cfg(not(feature = "with-moonbeam-runtime"))]
						return Err("Moonbeam runtime is not available. Please compile the node with `--features with-moonbeam-runtime` to enable it.".into());
					} else {
						#[cfg(feature = "with-moonbase-runtime")]
						{
							service::start_node::<service::moonbase_runtime::RuntimeApi, service::MoonbaseExecutor>(
								config,
								key,
								author_id,
								polkadot_config,
								id,
								collator,
								cli.run.sealing,
								cli.run.ethapi,
								rpc_params,
							)
							.await
							.map(|r| r.0)
							.map_err(Into::into)
						}
						#[cfg(not(feature = "with-moonbase-runtime"))]
						return Err("Moonbase runtime is not available. Please compile the node with `--features with-moonbase-runtime` to enable it.".into());
					}
				})
		}
	}
}

impl DefaultConfigurationValues for RelayChainCli {
	fn p2p_listen_port() -> u16 {
		30334
	}

	fn rpc_ws_listen_port() -> u16 {
		9945
	}

	fn rpc_http_listen_port() -> u16 {
		9934
	}

	fn prometheus_listen_port() -> u16 {
		9616
	}
}

impl CliConfiguration<Self> for RelayChainCli {
	fn shared_params(&self) -> &SharedParams {
		self.base.base.shared_params()
	}

	fn import_params(&self) -> Option<&ImportParams> {
		self.base.base.import_params()
	}

	fn network_params(&self) -> Option<&NetworkParams> {
		self.base.base.network_params()
	}

	fn keystore_params(&self) -> Option<&KeystoreParams> {
		self.base.base.keystore_params()
	}

	fn base_path(&self) -> Result<Option<BasePath>> {
		Ok(self
			.shared_params()
			.base_path()
			.or_else(|| self.base_path.clone().map(Into::into)))
	}

	fn rpc_http(&self, default_listen_port: u16) -> Result<Option<SocketAddr>> {
		self.base.base.rpc_http(default_listen_port)
	}

	fn rpc_ipc(&self) -> Result<Option<String>> {
		self.base.base.rpc_ipc()
	}

	fn rpc_ws(&self, default_listen_port: u16) -> Result<Option<SocketAddr>> {
		self.base.base.rpc_ws(default_listen_port)
	}

	fn prometheus_config(&self, default_listen_port: u16) -> Result<Option<PrometheusConfig>> {
		self.base.base.prometheus_config(default_listen_port)
	}

	fn init<C: SubstrateCli>(&self) -> Result<()> {
		unreachable!("PolkadotCli is never initialized; qed");
	}

	fn chain_id(&self, is_dev: bool) -> Result<String> {
		let chain_id = self.base.base.chain_id(is_dev)?;

		Ok(if chain_id.is_empty() {
			self.chain_id.clone().unwrap_or_default()
		} else {
			chain_id
		})
	}

	fn role(&self, is_dev: bool) -> Result<sc_service::Role> {
		self.base.base.role(is_dev)
	}

	fn transaction_pool(&self) -> Result<sc_service::config::TransactionPoolOptions> {
		self.base.base.transaction_pool()
	}

	fn state_cache_child_ratio(&self) -> Result<Option<usize>> {
		self.base.base.state_cache_child_ratio()
	}

	fn rpc_methods(&self) -> Result<sc_service::config::RpcMethods> {
		self.base.base.rpc_methods()
	}

	fn rpc_ws_max_connections(&self) -> Result<Option<usize>> {
		self.base.base.rpc_ws_max_connections()
	}

	fn rpc_cors(&self, is_dev: bool) -> Result<Option<Vec<String>>> {
		self.base.base.rpc_cors(is_dev)
	}

	fn telemetry_external_transport(&self) -> Result<Option<sc_service::config::ExtTransport>> {
		self.base.base.telemetry_external_transport()
	}

	fn default_heap_pages(&self) -> Result<Option<u64>> {
		self.base.base.default_heap_pages()
	}

	fn force_authoring(&self) -> Result<bool> {
		self.base.base.force_authoring()
	}

	fn disable_grandpa(&self) -> Result<bool> {
		self.base.base.disable_grandpa()
	}

	fn max_runtime_instances(&self) -> Result<Option<usize>> {
		self.base.base.max_runtime_instances()
	}

	fn announce_block(&self) -> Result<bool> {
		self.base.base.announce_block()
	}
}
