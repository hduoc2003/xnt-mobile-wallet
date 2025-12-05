use crate::api::export::Utxo;
use crate::application::json_rpc::core::api::rpc::RpcApi;
use crate::application::json_rpc::core::api::rpc::RpcResult;
use crate::application::json_rpc::core::model::block::RpcBlock;
use crate::application::json_rpc::core::model::common::RpcBlockSelector;
use crate::application::json_rpc::core::model::message::*;
use crate::application::json_rpc::server::rpc::RpcServer;
use crate::prelude::twenty_first::prelude::Mmr;
use crate::protocol::proof_abstractions::timestamp::Timestamp;
use crate::state::mempool::upgrade_priority::UpgradePriority;
use crate::state::wallet::address::view_key::ViewKey;
use crate::utxo_index::UtxoLocation;
use async_trait::async_trait;

#[async_trait]
impl RpcApi for RpcServer {
    async fn network_call(&self, _: NetworkRequest) -> RpcResult<NetworkResponse> {
        Ok(NetworkResponse {
            network: self.state.cli().network.to_string(),
        })
    }

    async fn height_call(&self, _: HeightRequest) -> RpcResult<HeightResponse> {
        let state = self.state.lock_guard().await;

        Ok(HeightResponse {
            height: state.chain.light_state().kernel.header.height.into(),
        })
    }

    async fn tip_digest_call(&self, _: TipDigestRequest) -> RpcResult<TipDigestResponse> {
        let state = self.state.lock_guard().await;
        let block = state.chain.light_state();

        Ok(TipDigestResponse {
            digest: block.hash(),
        })
    }

    async fn tip_call(&self, _: TipRequest) -> RpcResult<TipResponse> {
        let state = self.state.lock_guard().await;
        let block = state.chain.light_state();

        Ok(TipResponse {
            block: block.into(),
        })
    }

    async fn tip_proof_call(&self, _: TipProofRequest) -> RpcResult<TipProofResponse> {
        let state = self.state.lock_guard().await;
        let proof = &state.chain.light_state().proof;

        Ok(TipProofResponse {
            proof: proof.into(),
        })
    }

    async fn tip_kernel_call(&self, _: TipKernelRequest) -> RpcResult<TipKernelResponse> {
        let state = self.state.lock_guard().await;
        let kernel = &state.chain.light_state().kernel;

        Ok(TipKernelResponse {
            kernel: kernel.into(),
        })
    }

    async fn tip_header_call(&self, _: TipHeaderRequest) -> RpcResult<TipHeaderResponse> {
        let state = self.state.lock_guard().await;

        Ok(TipHeaderResponse {
            header: state.chain.light_state().header().into(),
        })
    }

    async fn tip_body_call(&self, _: TipBodyRequest) -> RpcResult<TipBodyResponse> {
        let state = self.state.lock_guard().await;

        Ok(TipBodyResponse {
            body: state.chain.light_state().body().into(),
        })
    }

    async fn tip_transaction_kernel_call(
        &self,
        _: TipTransactionKernelRequest,
    ) -> RpcResult<TipTransactionKernelResponse> {
        let state = self.state.lock_guard().await;

        Ok(TipTransactionKernelResponse {
            kernel: state.chain.light_state().body().transaction_kernel().into(),
        })
    }

    async fn tip_announcements_call(
        &self,
        _: TipAnnouncementsRequest,
    ) -> RpcResult<TipAnnouncementsResponse> {
        let state = self.state.lock_guard().await;

        Ok(TipAnnouncementsResponse {
            announcements: state
                .chain
                .light_state()
                .body()
                .transaction_kernel()
                .announcements
                .iter()
                .map(|a| a.message.clone().into())
                .collect(),
        })
    }

    async fn get_block_digest_call(
        &self,
        request: GetBlockDigestRequest,
    ) -> RpcResult<GetBlockDigestResponse> {
        let state = self.state.lock_guard().await;
        let digest = request.selector.as_digest(&state).await;

        Ok(GetBlockDigestResponse { digest })
    }

    async fn get_block_digests_call(
        &self,
        request: GetBlockDigestsRequest,
    ) -> RpcResult<GetBlockDigestsResponse> {
        let state = self.state.lock_guard().await;

        Ok(GetBlockDigestsResponse {
            digests: state
                .chain
                .archival_state()
                .block_height_to_block_digests(request.height.into())
                .await,
        })
    }

    async fn get_block_call(&self, request: GetBlockRequest) -> RpcResult<GetBlockResponse> {
        let state = self.state.lock_guard().await;

        let digest = request.selector.as_digest(&state).await;
        let block = match digest {
            Some(digest) => state
                .chain
                .archival_state()
                .get_block(digest)
                .await
                .unwrap()
                .as_ref()
                .map(RpcBlock::from),
            None => None,
        };

        Ok(GetBlockResponse { block })
    }

    async fn get_block_proof_call(
        &self,
        request: GetBlockProofRequest,
    ) -> RpcResult<GetBlockProofResponse> {
        let state = self.state.lock_guard().await;

        let digest = request.selector.as_digest(&state).await;
        let proof = match digest {
            Some(digest) => state
                .chain
                .archival_state()
                .get_block(digest)
                .await
                .unwrap()
                .as_ref()
                .map(|b| (&b.proof).into()),
            None => None,
        };

        Ok(GetBlockProofResponse { proof })
    }

    async fn get_block_kernel_call(
        &self,
        request: GetBlockKernelRequest,
    ) -> RpcResult<GetBlockKernelResponse> {
        let state = self.state.lock_guard().await;

        let digest = request.selector.as_digest(&state).await;
        let kernel = match digest {
            Some(digest) => state
                .chain
                .archival_state()
                .get_block(digest)
                .await
                .unwrap()
                .as_ref()
                .map(|b| (&b.kernel).into()),
            None => None,
        };

        Ok(GetBlockKernelResponse { kernel })
    }

    async fn get_block_header_call(
        &self,
        request: GetBlockHeaderRequest,
    ) -> RpcResult<GetBlockHeaderResponse> {
        let state = self.state.lock_guard().await;

        let digest = request.selector.as_digest(&state).await;
        let header = match digest {
            Some(digest) => state
                .chain
                .archival_state()
                .get_block(digest)
                .await
                .unwrap()
                .as_ref()
                .map(|b| b.header().into()),
            None => None,
        };

        Ok(GetBlockHeaderResponse { header })
    }

    async fn get_block_body_call(
        &self,
        request: GetBlockBodyRequest,
    ) -> RpcResult<GetBlockBodyResponse> {
        let state = self.state.lock_guard().await;

        let digest = request.selector.as_digest(&state).await;
        let body = match digest {
            Some(digest) => state
                .chain
                .archival_state()
                .get_block(digest)
                .await
                .unwrap()
                .as_ref()
                .map(|b| b.body().into()),
            None => None,
        };

        Ok(GetBlockBodyResponse { body })
    }

    async fn get_block_transaction_kernel_call(
        &self,
        request: GetBlockTransactionKernelRequest,
    ) -> RpcResult<GetBlockTransactionKernelResponse> {
        let state = self.state.lock_guard().await;

        let digest = request.selector.as_digest(&state).await;
        let kernel = match digest {
            Some(digest) => state
                .chain
                .archival_state()
                .get_block(digest)
                .await
                .unwrap()
                .as_ref()
                .map(|b| b.body().transaction_kernel().into()),
            None => None,
        };

        Ok(GetBlockTransactionKernelResponse { kernel })
    }

    async fn get_block_announcements_call(
        &self,
        request: GetBlockAnnouncementsRequest,
    ) -> RpcResult<GetBlockAnnouncementsResponse> {
        let state = self.state.lock_guard().await;

        let digest = request.selector.as_digest(&state).await;
        let announcements = match digest {
            Some(digest) => state
                .chain
                .archival_state()
                .get_block(digest)
                .await
                .unwrap()
                .as_ref()
                .map(|b| {
                    b.body()
                        .transaction_kernel()
                        .announcements
                        .iter()
                        .map(|a| a.message.clone().into())
                        .collect::<Vec<_>>()
                }),
            None => None,
        };

        Ok(GetBlockAnnouncementsResponse { announcements })
    }

    async fn get_block_announcements_range_call(
        &self,
        request: GetBlockAnnouncementsRangeRequest,
    ) -> RpcResult<GetBlockAnnouncementsRangeResponse> {
        let state = self.state.lock_guard().await;
        let mut blocks = Vec::new();

        // Limit range to prevent abuse (max 1000 blocks per request)
        let start_height = request.start_height;
        let end_height = std::cmp::min(
            request.end_height,
            request.start_height.saturating_add(1000),
        );

        for height in start_height..=end_height {
            use crate::protocol::consensus::block::block_height::BlockHeight;
            let block_height = BlockHeight::from(height);
            let selector = RpcBlockSelector::Height(block_height);
            let digest = selector.as_digest(&state).await;
            let announcements = match digest {
                Some(digest) => state
                    .chain
                    .archival_state()
                    .get_block(digest)
                    .await
                    .unwrap()
                    .as_ref()
                    .map(|b| {
                        b.body()
                            .transaction_kernel()
                            .announcements
                            .iter()
                            .map(|a| a.message.clone().into())
                            .collect::<Vec<_>>()
                    }),
                None => None,
            };

            blocks.push(BlockAnnouncementsEntry {
                height,
                announcements,
            });
        }

        Ok(GetBlockAnnouncementsRangeResponse { blocks })
    }

    async fn is_block_canonical_call(
        &self,
        request: IsBlockCanonicalRequest,
    ) -> RpcResult<IsBlockCanonicalResponse> {
        let state = self.state.lock_guard().await;

        Ok(IsBlockCanonicalResponse {
            canonical: state
                .chain
                .archival_state()
                .block_belongs_to_canonical_chain(request.digest)
                .await,
        })
    }

    async fn get_utxo_digest_call(
        &self,
        request: GetUtxoDigestRequest,
    ) -> RpcResult<GetUtxoDigestResponse> {
        let state = self.state.lock_guard().await;
        let aocl = &state.chain.archival_state().archival_mutator_set.ams().aocl;

        Ok(GetUtxoDigestResponse {
            digest: aocl.try_get_leaf(request.leaf_index).await,
        })
    }

    async fn find_utxo_origin_call(
        &self,
        request: FindUtxoOriginRequest,
    ) -> RpcResult<FindUtxoOriginResponse> {
        let allowed_search_depth = if self.unrestricted {
            request.search_depth
        } else {
            Some(request.search_depth.unwrap_or(100).min(100))
        };

        let state = self.state.lock_guard().await;
        let block = state
            .chain
            .archival_state()
            .find_canonical_block_with_output(request.addition_record.into(), allowed_search_depth)
            .await;

        Ok(FindUtxoOriginResponse {
            block: block.map(|block| block.hash()),
        })
    }

    async fn get_utxos_from_view_key_call(
        &self,
        request: GetUtxosFromViewKeyRequest,
    ) -> GetUtxosFromViewKeyResponse {
        use tasm_lib::prelude::Tip5;
        use tasm_lib::twenty_first::prelude::Mmr;

        let GetUtxosFromViewKeyRequest {
            view_key,
            start_block,
            end_block,
            max_search_depth,
        } = request;

        let view_key_hex = view_key.trim_start_matches("0x");
        let view_key_bytes = match hex::decode(view_key_hex) {
            Ok(bytes) => bytes,
            Err(e) => {
                return GetUtxosFromViewKeyResponse {
                    utxos: vec![],
                    error: Some(format!("Invalid view key hex: {}", e)),
                };
            }
        };

        let view_key: ViewKey = match bincode::deserialize(&view_key_bytes) {
            Ok(vk) => vk,
            Err(e) => {
                return GetUtxosFromViewKeyResponse {
                    utxos: vec![],
                    error: Some(format!("Failed to deserialize view key: {}", e)),
                };
            }
        };

        let receiver_identifier = view_key.receiver_identifier().value();

        let state = self.state.lock_guard().await;

        let current_tip_height = state.chain.light_state().header().height;

        let actual_end = request.end_block.unwrap_or(current_tip_height.value());
        let actual_end = actual_end.min(current_tip_height.value());

        if request.start_block > actual_end {
            return GetUtxosFromViewKeyResponse {
                utxos: vec![],
                error: Some(format!(
                    "Invalid block range: start_block ({}) > end_block ({})",
                    request.start_block, actual_end
                )),
            };
        }

        let indexed_locations = if let Some(ref index_db) = state.utxo_index {
            match receiver_identifier.try_into() {
                Ok(receiver_id_u64) => match index_db.get_utxos(receiver_id_u64).await {
                    Ok(Some(locations)) => {
                        let filtered: Vec<UtxoLocation> = locations
                            .into_iter()
                            .filter(|loc| {
                                loc.block_height >= start_block && loc.block_height <= actual_end
                            })
                            .collect();

                        if !filtered.is_empty() {
                            tracing::info!(
                                    "Using UTXO index: found {} potential UTXO(s) at specific blocks (fast path)",
                                    filtered.len()
                                );
                            Some(filtered)
                        } else {
                            tracing::info!("UTXO index: no UTXOs found in specified range");
                            Some(vec![])
                        }
                    }
                    Ok(None) => {
                        tracing::info!("UTXO index: no UTXOs found for this receiver");
                        Some(vec![])
                    }
                    Err(e) => {
                        tracing::warn!("UTXO index query failed: {}, falling back to full scan", e);
                        None
                    }
                },
                Err(e) => {
                    tracing::warn!("Failed to convert receiver_identifier to u64: {:?}, falling back to full scan", e);
                    None
                }
            }
        } else {
            tracing::info!("UTXO index not available, using full blockchain scan");
            None
        };

        let mut discovered_utxos = Vec::new();
        let archival_state = state.chain.archival_state();

        let ams = archival_state.archival_mutator_set.ams();

        let blocks_to_scan: Vec<u64> = if let Some(locations) = indexed_locations {
            let blocks: Vec<u64> = locations.iter().map(|loc| loc.block_height).collect();
            if blocks.is_empty() {
                tracing::info!("Index returned no blocks, returning empty result");
                return GetUtxosFromViewKeyResponse {
                    utxos: vec![],
                    error: None,
                };
            }
            blocks
        } else {
            tracing::info!(
                "Performing full blockchain scan from block {} to {}",
                start_block,
                actual_end
            );
            (start_block..=actual_end).collect()
        };

        for height in blocks_to_scan {
            let block_digest = match archival_state
                .archival_block_mmr
                .ammr()
                .try_get_leaf(height)
                .await
            {
                Some(digest) => digest,
                None => {
                    tracing::warn!("Block at height {} not found, stopping scan", height);
                    break;
                }
            };

            let block = match archival_state.get_block(block_digest).await {
                Ok(Some(block)) => block,
                Ok(None) => {
                    tracing::warn!("Block {} not found in archival state", block_digest);
                    continue;
                }
                Err(e) => {
                    tracing::warn!("Error reading block {}: {}", block_digest, e);
                    continue;
                }
            };

            let tx_kernel = block.kernel.body.transaction_kernel();

            if tx_kernel.outputs.is_empty() {
                continue;
            }

            let incoming_utxos = match view_key.scan_for_announced_utxos(tx_kernel) {
                Ok(utxos) => utxos,
                Err(e) => {
                    tracing::warn!("Failed to scan announcements in block {}: {}", height, e);
                    continue;
                }
            };

            if incoming_utxos.is_empty() {
                continue;
            }

            tracing::info!(
                "Found {} UTXO(s) in block height={}",
                incoming_utxos.len(),
                height
            );

            let block_msa_after = block
                .mutator_set_accumulator_after()
                .expect("Block must have mutator set after");
            let last_aocl_index_in_block = block_msa_after.aocl.num_leafs() - 1;

            let block_additions = block
                .mutator_set_update()
                .expect("Block must have mutator set update")
                .additions
                .len() as u64;
            let min_aocl_leaf_index = last_aocl_index_in_block - block_additions + 1;

            for incoming_utxo in incoming_utxos {
                let addition_record = incoming_utxo.addition_record();
                let item = Tip5::hash(incoming_utxo.utxo());

                let aocl_leaf_index = {
                    let mut haystack = last_aocl_index_in_block;
                    while ams.aocl.get_leaf_async(haystack).await
                        != addition_record.canonical_commitment
                    {
                        if haystack <= min_aocl_leaf_index {
                            tracing::warn!(
                                "UTXO addition record not found in block height={}",
                                height
                            );
                            break;
                        }
                        haystack -= 1;
                    }

                    if haystack < min_aocl_leaf_index {
                        continue;
                    }

                    haystack
                };

                let Ok(msmp) = ams
                    .restore_membership_proof(
                        item,
                        incoming_utxo.sender_randomness,
                        incoming_utxo.receiver_preimage,
                        aocl_leaf_index,
                    )
                    .await
                else {
                    tracing::debug!(
                        "Could not restore membership proof for UTXO at height {}",
                        height
                    );
                    continue;
                };

                let msa = ams.accumulator().await;
                if !msa.verify(item, &msmp) {
                    tracing::debug!("UTXO has been spent at height {}", height);
                    continue;
                }

                discovered_utxos.push(incoming_utxo);
            }
        }

        tracing::info!(
            "UTXO scan complete: found {} unspent UTXO(s), scanned blocks {}..={}",
            discovered_utxos.len(),
            request.start_block,
            actual_end
        );

        GetUtxosFromViewKeyResponse {
            utxos: discovered_utxos,
            error: None,
        }
    }

    async fn get_balance_from_view_key_call(
        &self,
        request: GetBalanceFromViewKeyRequest,
    ) -> GetBalanceFromViewKeyResponse {
        use crate::api::export::NativeCurrencyAmount;

        let utxos_response = self
            .get_utxos_from_view_key_call(GetUtxosFromViewKeyRequest {
                view_key: request.view_key.clone(),
                start_block: request.start_block,
                end_block: request.end_block,
                max_search_depth: request.max_search_depth,
            })
            .await;

        if let Some(error) = utxos_response.error {
            return GetBalanceFromViewKeyResponse {
                balance: "0".to_string(),
                available_balance: "0".to_string(),
                pending_balance: "0".to_string(),
                utxo_count: 0,
                error: Some(error),
            };
        }

        let current_timestamp = Timestamp::now();

        let total_balance: NativeCurrencyAmount = utxos_response
            .utxos
            .iter()
            .map(|incoming_utxo| incoming_utxo.utxo().get_native_currency_amount())
            .sum();

        let available_balance: NativeCurrencyAmount = utxos_response
            .utxos
            .iter()
            .filter(|incoming_utxo| incoming_utxo.utxo().can_spend_at(current_timestamp))
            .map(|incoming_utxo| incoming_utxo.utxo().get_native_currency_amount())
            .sum();

        let balance_str = total_balance.to_string();
        let available_balance_str = available_balance.to_string();

        let pending_balance = {
            let view_key_hex = request.view_key.trim_start_matches("0x");
            let view_key_bytes = match hex::decode(view_key_hex) {
                Ok(bytes) => bytes,
                Err(_) => {
                    return GetBalanceFromViewKeyResponse {
                        balance: balance_str,
                        available_balance: available_balance_str,
                        pending_balance: "0".to_string(),
                        utxo_count: utxos_response.utxos.len() as u64,
                        error: None,
                    };
                }
            };

            let view_key: ViewKey = match bincode::deserialize(&view_key_bytes) {
                Ok(vk) => vk,
                Err(_) => {
                    return GetBalanceFromViewKeyResponse {
                        balance: balance_str,
                        available_balance: available_balance_str,
                        pending_balance: "0".to_string(),
                        utxo_count: utxos_response.utxos.len() as u64,
                        error: None,
                    };
                }
            };

            let state = self.state.lock_guard().await;

            let pending_total: NativeCurrencyAmount = state
                .mempool
                .fee_density_iter()
                .filter_map(|(txid, _)| state.mempool.get(txid))
                .flat_map(|mempool_tx| {
                    view_key
                        .scan_for_announced_utxos(&mempool_tx.kernel)
                        .unwrap_or_default()
                        .into_iter()
                        .map(|pending_utxo| pending_utxo.utxo.get_native_currency_amount())
                })
                .sum();

            pending_total.to_string()
        };

        GetBalanceFromViewKeyResponse {
            balance: balance_str,
            available_balance: available_balance_str,
            pending_balance,
            utxo_count: utxos_response.utxos.len() as u64,
            error: None,
        }
    }

    async fn get_current_fee_call(&self, _: GetCurrentFeeRequest) -> GetCurrentFeeResponse {
        use crate::api::export::NativeCurrencyAmount;

        let state = self.state.lock_guard().await;
        let cli = state.cli();

        let min_relay_fee_per_input = cli.min_relay_pctx_fee_per_input;
        let min_relay_fee_str = min_relay_fee_per_input.to_string();

        let fees: Vec<NativeCurrencyAmount> = state
            .mempool
            .fee_density_iter()
            .filter_map(|(txid, _)| state.mempool.get(txid))
            .map(|tx| tx.kernel.fee)
            .collect();

        let mempool_tx_count = fees.len() as u64;

        let min_fee_from_mempool = fees.iter().min().copied();
        let max_fee_from_mempool = fees.iter().max().copied();

        GetCurrentFeeResponse {
            min_relay_fee_per_input: min_relay_fee_str,
            min_fee_from_mempool: min_fee_from_mempool.map(|f| f.to_string()),
            max_fee_from_mempool: max_fee_from_mempool.map(|f| f.to_string()),
            mempool_tx_count,
            error: None,
        }
    }

    async fn get_utxo_info_call(&self, request: GetUtxoInfoRequest) -> GetUtxoInfoResponse {
        use tasm_lib::prelude::Tip5;
        use tasm_lib::twenty_first::prelude::Mmr;

        let view_key_hex = request.view_key.trim_start_matches("0x");
        let view_key_bytes = match hex::decode(view_key_hex) {
            Ok(bytes) => bytes,
            Err(e) => {
                return GetUtxoInfoResponse {
                    utxos: vec![],
                    error: Some(format!("Invalid view key hex: {}", e)),
                };
            }
        };

        let view_key: ViewKey = match bincode::deserialize(&view_key_bytes) {
            Ok(vk) => vk,
            Err(e) => {
                return GetUtxoInfoResponse {
                    utxos: vec![],
                    error: Some(format!("Failed to deserialize view key: {}", e)),
                };
            }
        };

        let state = self.state.lock_guard().await;
        let current_tip_height = state.chain.light_state().header().height;
        let actual_end = request.end_block.unwrap_or(current_tip_height.value());
        let actual_end = actual_end.min(current_tip_height.value());

        if request.start_block > actual_end {
            return GetUtxoInfoResponse {
                utxos: vec![],
                error: Some(format!(
                    "Invalid block range: start_block ({}) > end_block ({})",
                    request.start_block, actual_end
                )),
            };
        }

        let receiver_identifier = view_key.receiver_identifier().value();
        let archival_state = state.chain.archival_state();
        let ams = archival_state.archival_mutator_set.ams();

        let indexed_locations = if let Some(ref index_db) = state.utxo_index {
            match receiver_identifier.try_into() {
                Ok(receiver_id_u64) => match index_db.get_utxos(receiver_id_u64).await {
                    Ok(Some(locations)) => {
                        let filtered: Vec<_> = locations
                            .into_iter()
                            .filter(|loc| {
                                loc.block_height >= request.start_block
                                    && loc.block_height <= actual_end
                            })
                            .collect();
                        if !filtered.is_empty() {
                            Some(filtered)
                        } else {
                            Some(vec![])
                        }
                    }
                    Ok(None) => Some(vec![]),
                    Err(_) => None,
                },
                Err(_) => None,
            }
        } else {
            None
        };

        let blocks_to_scan: Vec<u64> = if let Some(locations) = indexed_locations {
            if locations.is_empty() {
                return GetUtxoInfoResponse {
                    utxos: vec![],
                    error: None,
                };
            }
            locations.iter().map(|loc| loc.block_height).collect()
        } else {
            (request.start_block..=actual_end).collect()
        };

        let mut utxo_infos = Vec::new();

        for height in blocks_to_scan {
            let block_digest = match archival_state
                .archival_block_mmr
                .ammr()
                .try_get_leaf(height)
                .await
            {
                Some(digest) => digest,
                None => {
                    tracing::warn!("Block at height {} not found, stopping scan", height);
                    break;
                }
            };

            let block = match archival_state.get_block(block_digest).await {
                Ok(Some(block)) => block,
                Ok(None) => {
                    tracing::warn!("Block {} not found in archival state", block_digest);
                    continue;
                }
                Err(e) => {
                    tracing::warn!("Error reading block {}: {}", block_digest, e);
                    continue;
                }
            };

            let tx_kernel = block.kernel.body.transaction_kernel();

            if tx_kernel.outputs.is_empty() {
                continue;
            }

            let incoming_utxos = match view_key.scan_for_announced_utxos(tx_kernel) {
                Ok(utxos) => utxos,
                Err(e) => {
                    tracing::warn!("Failed to scan announcements in block {}: {}", height, e);
                    continue;
                }
            };

            if incoming_utxos.is_empty() {
                continue;
            }

            let block_msa_after = block
                .mutator_set_accumulator_after()
                .expect("Block must have mutator set after");
            let last_aocl_index_in_block = block_msa_after.aocl.num_leafs() - 1;

            let block_additions = block
                .mutator_set_update()
                .expect("Block must have mutator set update")
                .additions
                .len() as u64;
            let min_aocl_leaf_index = last_aocl_index_in_block - block_additions + 1;

            for incoming_utxo in incoming_utxos {
                let addition_record = incoming_utxo.addition_record();
                let item = Tip5::hash(incoming_utxo.utxo());

                let aocl_leaf_index = {
                    let mut haystack = last_aocl_index_in_block;
                    while ams.aocl.get_leaf_async(haystack).await
                        != addition_record.canonical_commitment
                    {
                        if haystack <= min_aocl_leaf_index {
                            tracing::warn!(
                                "UTXO addition record not found in block height={}",
                                height
                            );
                            break;
                        }
                        haystack -= 1;
                    }

                    if haystack < min_aocl_leaf_index {
                        continue;
                    }

                    haystack
                };

                // Restore the membership proof for this UTXO
                let Ok(_msmp) = ams
                    .restore_membership_proof(
                        item,
                        incoming_utxo.sender_randomness,
                        incoming_utxo.receiver_preimage,
                        aocl_leaf_index,
                    )
                    .await
                else {
                    tracing::debug!(
                        "Could not restore membership proof for UTXO at height {}",
                        height
                    );
                    continue;
                };

                let msa = ams.accumulator().await;
                if !msa.verify(item, &_msmp) {
                    tracing::debug!("UTXO has been spent at height {}", height);
                    continue;
                }

                let amount = incoming_utxo.utxo.get_native_currency_amount();
                let addition_record_hex = match bincode::serialize(&addition_record) {
                    Ok(bytes) => format!("0x{}", hex::encode(bytes)),
                    Err(e) => {
                        tracing::warn!("Failed to serialize addition record: {}", e);
                        continue;
                    }
                };

                let utxo_hash = format!("0x{}", item.to_hex());

                utxo_infos.push(
                    crate::application::json_rpc::core::model::message::UtxoInfo {
                        amount: amount.to_string(),
                        block_height: height,
                        addition_record: addition_record_hex,
                        utxo_hash,
                    },
                );
            }
        }

        GetUtxoInfoResponse {
            utxos: utxo_infos,
            error: None,
        }
    }

    async fn get_archival_mutator_set_call(
        &self,
        _: GetArchivalMutatorSetRequest,
    ) -> GetArchivalMutatorSetResponse {
        let state = self.state.lock_guard().await;
        let archival_mutator_set = state.chain.archival_state().archival_mutator_set.ams();
        let accumulator = archival_mutator_set.accumulator().await;
        GetArchivalMutatorSetResponse {
            archival_mutator_set: accumulator,
        }
    }

    async fn get_utxo_creation_block_call(
        &self,
        request: GetUtxoCreationBlockRequest,
    ) -> GetUtxoCreationBlockResponse {
        use crate::util_types::mutator_set::addition_record::AdditionRecord;

        let bytes = match hex::decode(request.addition_record.trim_start_matches("0x")) {
            Ok(b) => b,
            Err(e) => {
                return GetUtxoCreationBlockResponse {
                    last_aocl_index_in_block: Some(0),
                    num_outputs_in_block: Some(0),
                    error: Some(format!("Invalid addition_record hex: {}", e)),
                };
            }
        };

        let addition_record: AdditionRecord = match bincode::deserialize(&bytes) {
            Ok(ar) => ar,
            Err(e) => {
                return GetUtxoCreationBlockResponse {
                    last_aocl_index_in_block: Some(0),
                    num_outputs_in_block: Some(0),
                    error: Some(format!("Failed to deserialize addition_record: {}", e)),
                };
            }
        };

        let state = self.state.lock_guard().await;
        let max = Some(request.max_search_depth);
        let opt_block = state
            .chain
            .archival_state()
            .find_canonical_block_with_output(addition_record, max)
            .await;
        match opt_block {
            Some(block) => GetUtxoCreationBlockResponse {
                last_aocl_index_in_block: Some(
                    block
                        .mutator_set_accumulator_after()
                        .unwrap()
                        .aocl
                        .num_leafs()
                        - 1,
                ),
                num_outputs_in_block: Some(
                    block
                        .mutator_set_update()
                        .unwrap()
                        .additions
                        .len()
                        .try_into()
                        .unwrap(),
                ),
                error: None,
            },
            None => GetUtxoCreationBlockResponse {
                last_aocl_index_in_block: Some(0),
                num_outputs_in_block: Some(0),
                error: Some("UTXO not found in canonical chain".to_string()),
            },
        }
    }

    async fn get_aocl_authentication_path_call(
        &self,
        request: GetAoclAuthenticationPathRequest,
    ) -> GetAoclAuthenticationPathResponse {
        let state = self.state.lock_guard().await;
        let ams = state.chain.archival_state().archival_mutator_set.ams();

        match ams.get_aocl_authentication_path(request.aocl_leaf_index).await {
            Ok(auth_path) => {
                match bincode::serialize(&auth_path) {
                    Ok(bytes) => GetAoclAuthenticationPathResponse {
                        auth_path_hex: Some(format!("0x{}", hex::encode(bytes))),
                        error: None,
                    },
                    Err(e) => GetAoclAuthenticationPathResponse {
                        auth_path_hex: None,
                        error: Some(format!("Failed to serialize auth path: {}", e)),
                    },
                }
            }
            Err(e) => GetAoclAuthenticationPathResponse {
                auth_path_hex: None,
                error: Some(e.to_string()),
            },
        }
    }

    async fn get_chunks_and_auth_paths_call(
        &self,
        request: GetChunksAndAuthPathsRequest,
    ) -> GetChunksAndAuthPathsResponse {
        let state = self.state.lock_guard().await;
        let ams = state.chain.archival_state().archival_mutator_set.ams();

        let mut chunks = Vec::new();
        for chunk_index in request.chunk_indices {
            match ams.get_chunk_and_auth_path(chunk_index).await {
                Ok((auth_path, chunk)) => {
                    let chunk_hex = match bincode::serialize(&chunk) {
                        Ok(bytes) => format!("0x{}", hex::encode(bytes)),
                        Err(e) => {
                            return GetChunksAndAuthPathsResponse {
                                chunks: vec![],
                                error: Some(format!("Failed to serialize chunk {}: {}", chunk_index, e)),
                            };
                        }
                    };
                    let auth_path_hex = match bincode::serialize(&auth_path) {
                        Ok(bytes) => format!("0x{}", hex::encode(bytes)),
                        Err(e) => {
                            return GetChunksAndAuthPathsResponse {
                                chunks: vec![],
                                error: Some(format!("Failed to serialize auth path for chunk {}: {}", chunk_index, e)),
                            };
                        }
                    };
                    chunks.push(ChunkAndAuthPath {
                        chunk_index,
                        chunk_hex,
                        auth_path_hex,
                    });
                }
                Err(e) => {
                    return GetChunksAndAuthPathsResponse {
                        chunks: vec![],
                        error: Some(format!("Failed to get chunk {}: {}", chunk_index, e)),
                    };
                }
            }
        }

        GetChunksAndAuthPathsResponse {
            chunks,
            error: None,
        }
    }

    async fn generate_membership_proof_call(
        &self,
        request: GenerateMembershipProofRequest,
    ) -> GenerateMembershipProofResponse {
        let decode_hex = |s: &str| -> Result<Vec<u8>, String> {
            hex::decode(s.trim_start_matches("0x")).map_err(|e| format!("Invalid hex: {}", e))
        };

        let item_bytes = match decode_hex(&request.item_hex) {
            Ok(b) => b,
            Err(e) => {
                return GenerateMembershipProofResponse {
                    msmp_hex: None,
                    error: Some(e),
                }
            }
        };
        let sender_rand_bytes = match decode_hex(&request.sender_randomness_hex) {
            Ok(b) => b,
            Err(e) => {
                return GenerateMembershipProofResponse {
                    msmp_hex: None,
                    error: Some(e),
                }
            }
        };
        let receiver_preimage_bytes = match decode_hex(&request.receiver_preimage_hex) {
            Ok(b) => b,
            Err(e) => {
                return GenerateMembershipProofResponse {
                    msmp_hex: None,
                    error: Some(e),
                }
            }
        };

        let item = match bincode::deserialize(&item_bytes) {
            Ok(v) => v,
            Err(e) => {
                return GenerateMembershipProofResponse {
                    msmp_hex: None,
                    error: Some(format!("Failed to deserialize item: {}", e)),
                };
            }
        };
        let sender_randomness = match bincode::deserialize(&sender_rand_bytes) {
            Ok(v) => v,
            Err(e) => {
                return GenerateMembershipProofResponse {
                    msmp_hex: None,
                    error: Some(format!("Failed to deserialize sender_randomness: {}", e)),
                };
            }
        };
        let receiver_preimage = match bincode::deserialize(&receiver_preimage_bytes) {
            Ok(v) => v,
            Err(e) => {
                return GenerateMembershipProofResponse {
                    msmp_hex: None,
                    error: Some(format!("Failed to deserialize receiver_preimage: {}", e)),
                };
            }
        };

        let state = self.state.lock_guard().await;
        let ams = state.chain.archival_state().archival_mutator_set.ams();
        let aocl_leaf_index = request.aocl_leaf_index;

        let msmp = match ams
            .restore_membership_proof(item, sender_randomness, receiver_preimage, aocl_leaf_index)
            .await
        {
            Ok(p) => p,
            Err(e) => {
                return GenerateMembershipProofResponse {
                    msmp_hex: None,
                    error: Some(format!("Failed to restore membership proof: {}", e)),
                };
            }
        };

        let msmp_hex = match bincode::serialize(&msmp) {
            Ok(bytes) => format!("0x{}", hex::encode(bytes)),
            Err(e) => {
                return GenerateMembershipProofResponse {
                    msmp_hex: None,
                    error: Some(format!("Failed to serialize membership proof: {}", e)),
                };
            }
        };

        GenerateMembershipProofResponse {
            msmp_hex: Some(msmp_hex),
            error: None,
        }
    }

    async fn get_state_call(&self, _: GetStateRequest) -> GetStateResponse {
        use crate::protocol::proof_abstractions::timestamp::Timestamp;

        let state = self.state.lock_guard().await;

        let height = state.chain.light_state().kernel.header.height;

        let cli = state.cli().clone();

        let tip_digest = state.chain.light_state().hash();
        let tip_hash = format!(
            "0x{}",
            hex::encode(bincode::serialize(&tip_digest).unwrap_or_default())
        );
        let network = self.state.cli().network.to_string();

        let mutator_set_accumulator = state
            .chain
            .light_state()
            .mutator_set_accumulator_after()
            .expect("block in state must have mutator set after");
        let wallet_status = state
            .wallet_state
            .get_wallet_status(tip_digest, &mutator_set_accumulator)
            .await;

        let timestamp = Timestamp::now();
        let confirmed_available = state
            .wallet_state
            .confirmed_available_balance(&wallet_status, timestamp);
        let confirmed_total = state.wallet_state.confirmed_total_balance(&wallet_status);
        let unconfirmed_available = state
            .wallet_state
            .unconfirmed_available_balance(&wallet_status, timestamp);
        let unconfirmed_total = state.wallet_state.unconfirmed_total_balance(&wallet_status);

        GetStateResponse {
            state: StateInfo {
                height,
                tip_hash,
                network,
                wallet_balance: WalletBalanceInfo {
                    confirmed_available: confirmed_available.to_string(),
                    confirmed_total: confirmed_total.to_string(),
                    unconfirmed_available: unconfirmed_available.to_string(),
                    unconfirmed_total: unconfirmed_total.to_string(),
                },
            },
        }
    }

    async fn generate_utxo_with_proof_call(
        &self,
        request: GenerateUtxoWithProofRequest,
    ) -> GenerateUtxoWithProofResponse {
        use crate::twenty_first::prelude::Tip5;
        use crate::util_types::mutator_set::addition_record::AdditionRecord;

        let decode_hex = |s: &str| -> Result<Vec<u8>, String> {
            hex::decode(s.trim_start_matches("0x")).map_err(|e| format!("Invalid hex: {}", e))
        };

        let utxo_bytes = match decode_hex(&request.utxo_hex) {
            Ok(b) => b,
            Err(e) => {
                return GenerateUtxoWithProofResponse {
                    utxo_hex: None,
                    msmp_hex: None,
                    error: Some(e),
                }
            }
        };
        let addition_record_bytes = match decode_hex(&request.addition_record_hex) {
            Ok(b) => b,
            Err(e) => {
                return GenerateUtxoWithProofResponse {
                    utxo_hex: None,
                    msmp_hex: None,
                    error: Some(e),
                }
            }
        };
        let sender_rand_bytes = match decode_hex(&request.sender_randomness_hex) {
            Ok(b) => b,
            Err(e) => {
                return GenerateUtxoWithProofResponse {
                    utxo_hex: None,
                    msmp_hex: None,
                    error: Some(e),
                }
            }
        };
        let receiver_preimage_bytes = match decode_hex(&request.receiver_preimage_hex) {
            Ok(b) => b,
            Err(e) => {
                return GenerateUtxoWithProofResponse {
                    utxo_hex: None,
                    msmp_hex: None,
                    error: Some(e),
                }
            }
        };

        let utxo: Utxo = match bincode::deserialize(&utxo_bytes) {
            Ok(v) => v,
            Err(e) => {
                return GenerateUtxoWithProofResponse {
                    utxo_hex: None,
                    msmp_hex: None,
                    error: Some(format!("Failed to deserialize utxo: {}", e)),
                }
            }
        };
        let addition_record: AdditionRecord = match bincode::deserialize(&addition_record_bytes) {
            Ok(v) => v,
            Err(e) => {
                return GenerateUtxoWithProofResponse {
                    utxo_hex: None,
                    msmp_hex: None,
                    error: Some(format!("Failed to deserialize addition_record: {}", e)),
                }
            }
        };
        let sender_randomness = match bincode::deserialize(&sender_rand_bytes) {
            Ok(v) => v,
            Err(e) => {
                return GenerateUtxoWithProofResponse {
                    utxo_hex: None,
                    msmp_hex: None,
                    error: Some(format!("Failed to deserialize sender_randomness: {}", e)),
                }
            }
        };
        let receiver_preimage = match bincode::deserialize(&receiver_preimage_bytes) {
            Ok(v) => v,
            Err(e) => {
                return GenerateUtxoWithProofResponse {
                    utxo_hex: None,
                    msmp_hex: None,
                    error: Some(format!("Failed to deserialize receiver_preimage: {}", e)),
                }
            }
        };

        let state = self.state.lock_guard().await;
        let archival = state.chain.archival_state();
        let max = request.max_search_depth.or(Some(1000));
        let Some(creation_block) = archival
            .find_canonical_block_with_output(addition_record, max)
            .await
        else {
            return GenerateUtxoWithProofResponse {
                utxo_hex: None,
                msmp_hex: None,
                error: Some("UTXO not found in canonical chain".to_string()),
            };
        };

        let last_aocl_index_in_block = creation_block
            .mutator_set_accumulator_after()
            .expect("Block must have mutator set after")
            .aocl
            .num_leafs()
            - 1;
        let num_outputs_in_block: u64 = creation_block
            .mutator_set_update()
            .expect("Block must have mutator set update")
            .additions
            .len()
            .try_into()
            .unwrap();
        let min_aocl_leaf_index = last_aocl_index_in_block - num_outputs_in_block + 1;

        let mut haystack = last_aocl_index_in_block;
        let ams = archival.archival_mutator_set.ams();
        while ams.aocl.get_leaf_async(haystack).await != addition_record.canonical_commitment {
            if haystack <= min_aocl_leaf_index {
                return GenerateUtxoWithProofResponse {
                    utxo_hex: None,
                    msmp_hex: None,
                    error: Some("UTXO addition record not found in block".to_string()),
                };
            }
            haystack -= 1;
        }
        let aocl_leaf_index = haystack;

        let item = Tip5::hash(&utxo);
        let msmp = match ams
            .restore_membership_proof(item, sender_randomness, receiver_preimage, aocl_leaf_index)
            .await
        {
            Ok(p) => p,
            Err(e) => {
                return GenerateUtxoWithProofResponse {
                    utxo_hex: None,
                    msmp_hex: None,
                    error: Some(format!("Failed to restore membership proof: {}", e)),
                };
            }
        };

        let utxo_hex = format!("0x{}", hex::encode(bincode::serialize(&utxo).unwrap()));
        let msmp_hex = format!("0x{}", hex::encode(bincode::serialize(&msmp).unwrap()));

        GenerateUtxoWithProofResponse {
            utxo_hex: Some(utxo_hex),
            msmp_hex: Some(msmp_hex),
            error: None,
        }
    }

    async fn submit_transaction_call(
        &self,
        request: SubmitTransactionRequest,
    ) -> SubmitTransactionResponse {
        use crate::protocol::consensus::transaction::Transaction;

        tracing::info!("Submitting pre-built transaction from client");

        let tx_bytes = match hex::decode(request.transaction_hex.trim_start_matches("0x")) {
            Ok(bytes) => bytes,
            Err(e) => {
                return SubmitTransactionResponse {
                    txid: String::new(),
                    success: false,
                    error: Some(format!("Invalid transaction hex: {}", e)),
                };
            }
        };

        let transaction: Transaction = match bincode::deserialize(&tx_bytes) {
            Ok(tx) => tx,
            Err(e) => {
                return SubmitTransactionResponse {
                    txid: String::new(),
                    success: false,
                    error: Some(format!("Failed to deserialize transaction: {}", e)),
                };
            }
        };

        // Verify the transaction is valid
        let consensus_rule_set = {
            let state = self.state.lock_guard().await;
            let height = state.chain.light_state().kernel.header.height;
            crate::protocol::consensus::consensus_rule_set::ConsensusRuleSet::infer_from(
                self.state.cli().network,
                height,
            )
        };

        let is_valid = transaction
            .is_valid(self.state.cli().network, consensus_rule_set)
            .await;

        if !is_valid {
            tracing::warn!("Transaction validation failed");
            let txid = transaction.kernel.txid();
            return SubmitTransactionResponse {
                txid: format!("0x{}", hex::encode(bincode::serialize(&txid).unwrap())),
                success: false,
                error: Some("Transaction validation failed".to_string()),
            };
        }

        let txid = transaction.kernel.txid();
        let txid_hex = format!("0x{}", hex::encode(bincode::serialize(&txid).unwrap()));

        tracing::info!(
            "Transaction {} validated successfully, adding to mempool",
            txid_hex
        );

        use std::sync::Arc;
        let transaction_arc = Arc::new(transaction.clone());

        let mut state = self.clone();
        {
            let mut gsm = state.state.lock_guard_mut().await;
            gsm.mempool_insert(transaction, UpgradePriority::Irrelevant)
                .await;
        }

        let broadcaster_tx = self.state.rpc_server_to_main_tx();
        if broadcaster_tx
            .send(crate::application::loops::channel::RPCServerToMain::BroadcastTx(transaction_arc))
            .await
            .is_err()
        {
            tracing::warn!("Failed to broadcast transaction {} to peers", txid_hex);
        }

        tracing::info!("Transaction {} added to mempool successfully", txid_hex);

        SubmitTransactionResponse {
            txid: txid_hex,
            success: true,
            error: None,
        }
    }

    async fn get_xnt_price_call(&self, _: GetXntPriceRequest) -> GetXntPriceResponse {
        GetXntPriceResponse {
            price: "4.0".to_string(),
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
pub mod tests {
    use std::collections::HashSet;

    use macro_rules_attr::apply;
    use tasm_lib::prelude::Digest;

    use crate::api::export::Network;
    use crate::application::config::cli_args;
    use crate::application::json_rpc::core::api::rpc::RpcApi;
    use crate::application::json_rpc::core::model::common::RpcBlockSelector;
    use crate::application::json_rpc::server::rpc::RpcServer;
    use crate::protocol::consensus::block::block_height::BlockHeight;
    use crate::protocol::consensus::transaction::Transaction;
    use crate::protocol::consensus::transaction::TransactionProof;
    use crate::state::wallet::wallet_entropy::WalletEntropy;
    use crate::tests::shared::blocks::invalid_block_with_transaction;
    use crate::tests::shared::globalstate::mock_genesis_global_state;
    use crate::tests::shared::strategies::txkernel;
    use crate::tests::shared_tokio_runtime;
    use crate::twenty_first::bfe;
    use crate::BFieldElement;
    use crate::Block;

    pub async fn test_rpc_server() -> RpcServer {
        let global_state_lock = mock_genesis_global_state(
            2,
            WalletEntropy::new_random(),
            cli_args::Args::default_with_network(Network::Main),
        )
        .await;

        RpcServer::new(global_state_lock, None)
    }

    #[apply(shared_tokio_runtime)]
    async fn network_is_consistent() {
        let rpc_server = test_rpc_server().await;
        assert_eq!("main", rpc_server.network().await.unwrap().network);
    }

    #[apply(shared_tokio_runtime)]
    async fn height_is_correct() {
        let rpc_server = test_rpc_server().await;
        assert_eq!(bfe!(0), rpc_server.height().await.unwrap().height);
    }

    #[test_strategy::proptest(async = "tokio", cases = 5)]
    async fn tip_calls_are_consistent(
        #[strategy(txkernel::with_lengths(0, 2, 2, true))]
        tx_block1: crate::protocol::consensus::transaction::transaction_kernel::TransactionKernel,
    ) {
        let mut rpc_server = test_rpc_server().await;

        let tx_block1 = Transaction {
            kernel: tx_block1,
            proof: TransactionProof::invalid(),
        };
        let block1 = invalid_block_with_transaction(&Block::genesis(Network::Main), tx_block1);
        rpc_server.state.set_new_tip(block1.clone()).await.unwrap();

        let digest = rpc_server.tip_digest().await.unwrap().digest;
        assert_eq!(block1.hash(), digest);

        let block = rpc_server.tip().await.unwrap().block;
        let proof = rpc_server.tip_proof().await.unwrap().proof;
        assert_eq!(block.proof, proof);

        let kernel = rpc_server.tip_kernel().await.unwrap().kernel;
        let header = rpc_server.tip_header().await.unwrap().header;
        assert_eq!(kernel.header, header);

        let body = rpc_server.tip_body().await.unwrap().body;
        let transaction_kernel = rpc_server.tip_transaction_kernel().await.unwrap().kernel;
        assert_eq!(body.transaction_kernel, transaction_kernel);

        let announcements = rpc_server.tip_announcements().await.unwrap().announcements;
        assert_eq!(transaction_kernel.announcements, announcements);
    }

    #[test_strategy::proptest(async = "tokio", cases = 5)]
    async fn get_block_calls_are_consistent(
        #[strategy(0usize..8)] _num_announcements: usize,
        #[strategy(txkernel::with_lengths(0, 2, #_num_announcements, true))]
    tx_block1: crate::protocol::consensus::transaction::transaction_kernel::TransactionKernel,
    ) {
        let mut rpc_server = test_rpc_server().await;

        let tx_block1 = Transaction {
            kernel: tx_block1,
            proof: TransactionProof::invalid(),
        };
        let block1 = invalid_block_with_transaction(&Block::genesis(Network::Main), tx_block1);
        rpc_server.state.set_new_tip(block1.clone()).await.unwrap();

        for height in [BlockHeight::genesis(), BlockHeight::genesis().next()] {
            let height_selector = RpcBlockSelector::Height(height);
            let digest = rpc_server
                .get_block_digest(height_selector)
                .await
                .unwrap()
                .digest
                .expect("digest should be available");
            let selector = RpcBlockSelector::Digest(digest);

            let block = rpc_server
                .get_block(selector)
                .await
                .unwrap()
                .block
                .expect("block should exist");
            let proof = rpc_server
                .get_block_proof(selector)
                .await
                .unwrap()
                .proof
                .expect("proof should exist");
            assert_eq!(block.proof, proof);

            let kernel = rpc_server
                .get_block_kernel(selector)
                .await
                .unwrap()
                .kernel
                .expect("kernel should exist");
            let header = rpc_server
                .get_block_header(selector)
                .await
                .unwrap()
                .header
                .expect("header should exist");
            assert_eq!(kernel.header, header);
            assert_eq!(header.height, height.into());

            let body = rpc_server
                .get_block_body(selector)
                .await
                .unwrap()
                .body
                .expect("body should exist");
            let transaction_kernel = rpc_server
                .get_block_transaction_kernel(selector)
                .await
                .unwrap()
                .kernel
                .expect("transaction kernel should exist");
            assert_eq!(body.transaction_kernel, transaction_kernel);

            let announcements = rpc_server
                .get_block_announcements(selector)
                .await
                .unwrap()
                .announcements
                .expect("announcements should exist");
            assert_eq!(transaction_kernel.announcements, announcements);
        }
    }

    #[test_strategy::proptest(async = "tokio", cases = 5)]
    async fn get_block_digests_returns_competing_blocks(
        #[strategy(txkernel::with_lengths(0, 2, 2, true))]
    tx_block1: crate::protocol::consensus::transaction::transaction_kernel::TransactionKernel,
        #[strategy(txkernel::with_lengths(0, 2, 2, true))]
    tx_block2: crate::protocol::consensus::transaction::transaction_kernel::TransactionKernel,
    ) {
        let mut rpc_server = test_rpc_server().await;

        let tx_block1 = Transaction {
            kernel: tx_block1,
            proof: TransactionProof::invalid(),
        };
        let tx_block2 = Transaction {
            kernel: tx_block2,
            proof: TransactionProof::invalid(),
        };
        let block1 = invalid_block_with_transaction(&Block::genesis(Network::Main), tx_block1);
        let block2 = invalid_block_with_transaction(&Block::genesis(Network::Main), tx_block2);
        rpc_server.state.set_new_tip(block1.clone()).await.unwrap();
        rpc_server.state.set_new_tip(block2.clone()).await.unwrap();

        let digests = rpc_server
            .get_block_digests(BlockHeight::genesis().next().into())
            .await
            .unwrap()
            .digests;

        let expected: HashSet<_> = [block1.hash(), block2.hash()].into();
        let actual: HashSet<_> = digests.into_iter().collect();

        assert_eq!(expected, actual);
    }

    #[apply(shared_tokio_runtime)]
    async fn is_block_canonical_consistency() {
        let rpc_server = test_rpc_server().await;

        // Test genesis block is canonical
        let genesis_digest = rpc_server.tip_digest().await.unwrap().digest;
        let is_genesis_canonical = rpc_server
            .is_block_canonical(genesis_digest)
            .await
            .unwrap()
            .canonical;
        assert!(is_genesis_canonical, "Genesis block should be canonical");

        // Test non-existent block is not canonical
        let fake_digest = Digest::default();
        let is_fake_canonical = rpc_server
            .is_block_canonical(fake_digest)
            .await
            .unwrap()
            .canonical;
        assert!(
            !is_fake_canonical,
            "Non-existent block should not be canonical"
        );
    }

    #[test_strategy::proptest(async = "tokio", cases = 5)]
    async fn utxo_calls_are_consistent(
        #[strategy(0usize..8)] _num_outputs: usize,
        #[strategy(txkernel::with_lengths(0usize, #_num_outputs, 0usize, true))]
        transaction_kernel: crate::protocol::consensus::transaction::transaction_kernel::TransactionKernel,
    ) {
        let mut rpc_server = test_rpc_server().await;

        // Before new block check size of aocl leaves so we can know exact index of new outputs
        let num_aocl_leaves = rpc_server
            .state
            .lock_guard()
            .await
            .chain
            .archival_state()
            .archival_mutator_set
            .ams()
            .aocl
            .num_leafs()
            .await;

        let transaction = Transaction {
            kernel: transaction_kernel,
            proof: TransactionProof::invalid(),
        };
        let block = invalid_block_with_transaction(&Block::genesis(Network::Main), transaction);
        rpc_server.state.set_new_tip(block.clone()).await.unwrap();

        for (i, output) in block.body().transaction_kernel().outputs.iter().enumerate() {
            let utxo_index = num_aocl_leaves + i as u64;
            let digest_entry = rpc_server
                .get_utxo_digest(utxo_index)
                .await
                .expect("failed to get utxo digest");

            let digest = digest_entry.digest.expect("missing digest for utxo output");

            assert_eq!(
                output.canonical_commitment, digest,
                "canonical commitment mismatch for utxo at index {utxo_index}"
            );

            // Check origin of UTXO
            let origin_response = rpc_server
                .find_utxo_origin((*output).into(), None)
                .await
                .expect("find_utxo_origin RPC failed");

            let origin_block = origin_response.block;
            assert!(
                origin_block.is_some(),
                "expected origin block for utxo {utxo_index}"
            );
            assert_eq!(
                origin_block.unwrap(),
                block.hash(),
                "origin block mismatch for utxo {utxo_index}"
            );
        }
    }
}
