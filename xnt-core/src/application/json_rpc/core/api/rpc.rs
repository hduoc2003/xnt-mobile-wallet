use async_trait::async_trait;
use serde::Deserialize;
use serde::Serialize;
use tasm_lib::prelude::Digest;
use tasm_lib::triton_vm::prelude::BFieldElement;
use thiserror::Error;

use crate::application::json_rpc::core::model::block::transaction_kernel::RpcAdditionRecord;
use crate::application::json_rpc::core::model::common::RpcBlockSelector;
use crate::application::json_rpc::core::model::json::JsonError;
use crate::application::json_rpc::core::model::message::*;

#[derive(Debug, Clone, Error, Eq, PartialEq, Serialize, Deserialize)]
pub enum RpcError {
    #[error("JSON-RPC server error: {0}")]
    Server(JsonError),
}

pub type RpcResult<T> = Result<T, RpcError>;

#[async_trait]
pub trait RpcApi: Sync + Send {
    /* Node */

    async fn network(&self) -> RpcResult<NetworkResponse> {
        self.network_call(NetworkRequest {}).await
    }
    async fn network_call(&self, request: NetworkRequest) -> RpcResult<NetworkResponse>;

    /* Chain */

    async fn height(&self) -> RpcResult<HeightResponse> {
        self.height_call(HeightRequest {}).await
    }
    async fn height_call(&self, request: HeightRequest) -> RpcResult<HeightResponse>;

    async fn tip_digest(&self) -> RpcResult<TipDigestResponse> {
        self.tip_digest_call(TipDigestRequest {}).await
    }
    async fn tip_digest_call(&self, request: TipDigestRequest) -> RpcResult<TipDigestResponse>;

    async fn tip(&self) -> RpcResult<TipResponse> {
        self.tip_call(TipRequest {}).await
    }
    async fn tip_call(&self, request: TipRequest) -> RpcResult<TipResponse>;

    async fn tip_proof(&self) -> RpcResult<TipProofResponse> {
        self.tip_proof_call(TipProofRequest {}).await
    }
    async fn tip_proof_call(&self, request: TipProofRequest) -> RpcResult<TipProofResponse>;

    async fn tip_kernel(&self) -> RpcResult<TipKernelResponse> {
        self.tip_kernel_call(TipKernelRequest {}).await
    }
    async fn tip_kernel_call(&self, request: TipKernelRequest) -> RpcResult<TipKernelResponse>;

    async fn tip_header(&self) -> RpcResult<TipHeaderResponse> {
        self.tip_header_call(TipHeaderRequest {}).await
    }
    async fn tip_header_call(&self, request: TipHeaderRequest) -> RpcResult<TipHeaderResponse>;

    async fn tip_body(&self) -> RpcResult<TipBodyResponse> {
        self.tip_body_call(TipBodyRequest {}).await
    }
    async fn tip_body_call(&self, request: TipBodyRequest) -> RpcResult<TipBodyResponse>;

    async fn tip_transaction_kernel(&self) -> RpcResult<TipTransactionKernelResponse> {
        self.tip_transaction_kernel_call(TipTransactionKernelRequest {})
            .await
    }
    async fn tip_transaction_kernel_call(
        &self,
        request: TipTransactionKernelRequest,
    ) -> RpcResult<TipTransactionKernelResponse>;

    async fn tip_announcements(&self) -> RpcResult<TipAnnouncementsResponse> {
        self.tip_announcements_call(TipAnnouncementsRequest {})
            .await
    }
    async fn tip_announcements_call(
        &self,
        request: TipAnnouncementsRequest,
    ) -> RpcResult<TipAnnouncementsResponse>;

    /* Archival */

    async fn get_block_digests(&self, height: BFieldElement) -> RpcResult<GetBlockDigestsResponse> {
        self.get_block_digests_call(GetBlockDigestsRequest { height })
            .await
    }
    async fn get_block_digests_call(
        &self,
        request: GetBlockDigestsRequest,
    ) -> RpcResult<GetBlockDigestsResponse>;

    async fn get_block_digest(
        &self,
        selector: RpcBlockSelector,
    ) -> RpcResult<GetBlockDigestResponse> {
        self.get_block_digest_call(GetBlockDigestRequest { selector })
            .await
    }
    async fn get_block_digest_call(
        &self,
        request: GetBlockDigestRequest,
    ) -> RpcResult<GetBlockDigestResponse>;

    async fn get_block(&self, selector: RpcBlockSelector) -> RpcResult<GetBlockResponse> {
        self.get_block_call(GetBlockRequest { selector }).await
    }
    async fn get_block_call(&self, request: GetBlockRequest) -> RpcResult<GetBlockResponse>;

    async fn get_block_proof(
        &self,
        selector: RpcBlockSelector,
    ) -> RpcResult<GetBlockProofResponse> {
        self.get_block_proof_call(GetBlockProofRequest { selector })
            .await
    }
    async fn get_block_proof_call(
        &self,
        request: GetBlockProofRequest,
    ) -> RpcResult<GetBlockProofResponse>;

    async fn get_block_kernel(
        &self,
        selector: RpcBlockSelector,
    ) -> RpcResult<GetBlockKernelResponse> {
        self.get_block_kernel_call(GetBlockKernelRequest { selector })
            .await
    }
    async fn get_block_kernel_call(
        &self,
        request: GetBlockKernelRequest,
    ) -> RpcResult<GetBlockKernelResponse>;

    async fn get_block_header(
        &self,
        selector: RpcBlockSelector,
    ) -> RpcResult<GetBlockHeaderResponse> {
        self.get_block_header_call(GetBlockHeaderRequest { selector })
            .await
    }
    async fn get_block_header_call(
        &self,
        request: GetBlockHeaderRequest,
    ) -> RpcResult<GetBlockHeaderResponse>;

    async fn get_block_body(&self, selector: RpcBlockSelector) -> RpcResult<GetBlockBodyResponse> {
        self.get_block_body_call(GetBlockBodyRequest { selector })
            .await
    }
    async fn get_block_body_call(
        &self,
        request: GetBlockBodyRequest,
    ) -> RpcResult<GetBlockBodyResponse>;

    async fn get_block_transaction_kernel(
        &self,
        selector: RpcBlockSelector,
    ) -> RpcResult<GetBlockTransactionKernelResponse> {
        self.get_block_transaction_kernel_call(GetBlockTransactionKernelRequest { selector })
            .await
    }
    async fn get_block_transaction_kernel_call(
        &self,
        request: GetBlockTransactionKernelRequest,
    ) -> RpcResult<GetBlockTransactionKernelResponse>;

    async fn get_block_announcements(
        &self,
        selector: RpcBlockSelector,
    ) -> RpcResult<GetBlockAnnouncementsResponse> {
        self.get_block_announcements_call(GetBlockAnnouncementsRequest { selector })
            .await
    }
    async fn get_block_announcements_call(
        &self,
        request: GetBlockAnnouncementsRequest,
    ) -> RpcResult<GetBlockAnnouncementsResponse>;

    async fn get_block_announcements_range(
        &self,
        start_height: u64,
        end_height: u64,
    ) -> RpcResult<GetBlockAnnouncementsRangeResponse> {
        self.get_block_announcements_range_call(GetBlockAnnouncementsRangeRequest {
            start_height,
            end_height,
        })
        .await
    }
    async fn get_block_announcements_range_call(
        &self,
        request: GetBlockAnnouncementsRangeRequest,
    ) -> RpcResult<GetBlockAnnouncementsRangeResponse>;

    async fn is_block_canonical(&self, digest: Digest) -> RpcResult<IsBlockCanonicalResponse> {
        self.is_block_canonical_call(IsBlockCanonicalRequest { digest })
            .await
    }
    async fn is_block_canonical_call(
        &self,
        request: IsBlockCanonicalRequest,
    ) -> RpcResult<IsBlockCanonicalResponse>;

    async fn get_utxo_digest(&self, leaf_index: u64) -> RpcResult<GetUtxoDigestResponse> {
        self.get_utxo_digest_call(GetUtxoDigestRequest { leaf_index })
            .await
    }
    async fn get_utxo_digest_call(
        &self,
        request: GetUtxoDigestRequest,
    ) -> RpcResult<GetUtxoDigestResponse>;

    async fn find_utxo_origin(
        &self,
        addition_record: RpcAdditionRecord,
        search_depth: Option<u64>,
    ) -> RpcResult<FindUtxoOriginResponse> {
        self.find_utxo_origin_call(FindUtxoOriginRequest {
            addition_record,
            search_depth,
        })
        .await
    }
    async fn find_utxo_origin_call(
        &self,
        request: FindUtxoOriginRequest,
    ) -> RpcResult<FindUtxoOriginResponse>;

    async fn get_utxos_from_view_key(
        &self,
        view_key: String,
        start_block: u64,
        end_block: Option<u64>,
        max_search_depth: u64,
    ) -> GetUtxosFromViewKeyResponse {
        self.get_utxos_from_view_key_call(GetUtxosFromViewKeyRequest {
            view_key,
            start_block,
            end_block,
            max_search_depth,
        })
        .await
    }
    async fn get_utxos_from_view_key_call(
        &self,
        request: GetUtxosFromViewKeyRequest,
    ) -> GetUtxosFromViewKeyResponse;

    async fn get_balance_from_view_key(
        &self,
        view_key: String,
        start_block: u64,
        end_block: Option<u64>,
        max_search_depth: u64,
    ) -> GetBalanceFromViewKeyResponse {
        self.get_balance_from_view_key_call(GetBalanceFromViewKeyRequest {
            view_key,
            start_block,
            end_block,
            max_search_depth,
        })
        .await
    }
    async fn get_balance_from_view_key_call(
        &self,
        request: GetBalanceFromViewKeyRequest,
    ) -> GetBalanceFromViewKeyResponse;

    async fn get_archival_mutator_set_call(
        &self,
        request: GetArchivalMutatorSetRequest,
    ) -> GetArchivalMutatorSetResponse;

    async fn get_utxo_creation_block(
        &self,
        addition_record: String,
        max_search_depth: u64,
    ) -> GetUtxoCreationBlockResponse {
        self.get_utxo_creation_block_call(GetUtxoCreationBlockRequest {
            addition_record,
            max_search_depth,
        })
        .await
    }
    async fn get_utxo_creation_block_call(
        &self,
        request: GetUtxoCreationBlockRequest,
    ) -> GetUtxoCreationBlockResponse;

    async fn get_aocl_authentication_path_call(
        &self,
        request: GetAoclAuthenticationPathRequest,
    ) -> GetAoclAuthenticationPathResponse;

    async fn get_chunks_and_auth_paths_call(
        &self,
        request: GetChunksAndAuthPathsRequest,
    ) -> GetChunksAndAuthPathsResponse;

    async fn generate_membership_proof(
        &self,
        item_hex: String,
        sender_randomness_hex: String,
        receiver_preimage_hex: String,
        aocl_leaf_index: u64,
    ) -> GenerateMembershipProofResponse {
        self.generate_membership_proof_call(GenerateMembershipProofRequest {
            item_hex,
            sender_randomness_hex,
            receiver_preimage_hex,
            aocl_leaf_index,
        })
        .await
    }
    async fn generate_membership_proof_call(
        &self,
        request: GenerateMembershipProofRequest,
    ) -> GenerateMembershipProofResponse;

    async fn generate_utxo_with_proof(
        &self,
        req: GenerateUtxoWithProofRequest,
    ) -> GenerateUtxoWithProofResponse {
        self.generate_utxo_with_proof_call(req).await
    }
    async fn generate_utxo_with_proof_call(
        &self,
        request: GenerateUtxoWithProofRequest,
    ) -> GenerateUtxoWithProofResponse;

    async fn get_state(&self) -> GetStateResponse {
        self.get_state_call(GetStateRequest {}).await
    }
    async fn get_state_call(&self, request: GetStateRequest) -> GetStateResponse;

    async fn submit_transaction(
        &self,
        request: SubmitTransactionRequest,
    ) -> SubmitTransactionResponse {
        self.submit_transaction_call(request).await
    }
    async fn submit_transaction_call(
        &self,
        request: SubmitTransactionRequest,
    ) -> SubmitTransactionResponse;

    async fn get_current_fee(&self) -> GetCurrentFeeResponse {
        self.get_current_fee_call(GetCurrentFeeRequest {}).await
    }
    async fn get_current_fee_call(&self, request: GetCurrentFeeRequest) -> GetCurrentFeeResponse;

    async fn get_utxo_info(&self, request: GetUtxoInfoRequest) -> GetUtxoInfoResponse {
        self.get_utxo_info_call(request).await
    }
    async fn get_utxo_info_call(&self, request: GetUtxoInfoRequest) -> GetUtxoInfoResponse;

    async fn get_xnt_price(&self) -> GetXntPriceResponse {
        self.get_xnt_price_call(GetXntPriceRequest {}).await
    }
    async fn get_xnt_price_call(&self, request: GetXntPriceRequest) -> GetXntPriceResponse;
}
