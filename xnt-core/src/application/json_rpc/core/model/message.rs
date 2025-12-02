use crate::api::export::BlockHeight;
use crate::application::json_rpc::core::model::block::body::*;
use crate::application::json_rpc::core::model::block::header::*;
use crate::application::json_rpc::core::model::block::transaction_kernel::*;
use crate::application::json_rpc::core::model::block::*;
use crate::application::json_rpc::core::model::common::*;
use crate::state::wallet::incoming_utxo::IncomingUtxo;
use crate::util_types::mutator_set::mutator_set_accumulator::MutatorSetAccumulator;
use serde::Deserialize;
use serde::Serialize;
use serde_tuple::Deserialize_tuple;
use serde_tuple::Serialize_tuple;
use tasm_lib::prelude::Digest;
use tasm_lib::triton_vm::prelude::BFieldElement;

#[derive(Clone, Copy, Debug, Serialize_tuple, Deserialize_tuple)]
#[serde(rename_all = "camelCase")]
pub struct NetworkRequest {}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkResponse {
    pub network: String,
}

#[derive(Clone, Copy, Debug, Serialize_tuple, Deserialize_tuple)]
#[serde(rename_all = "camelCase")]
pub struct HeightRequest {}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeightResponse {
    pub height: BFieldElement,
}

#[derive(Clone, Copy, Debug, Serialize_tuple, Deserialize_tuple)]
#[serde(rename_all = "camelCase")]
pub struct TipDigestRequest {}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TipDigestResponse {
    pub digest: Digest,
}

#[derive(Clone, Copy, Debug, Serialize_tuple, Deserialize_tuple)]
#[serde(rename_all = "camelCase")]
pub struct TipRequest {}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TipResponse {
    pub block: RpcBlock,
}

#[derive(Clone, Copy, Debug, Serialize_tuple, Deserialize_tuple)]
#[serde(rename_all = "camelCase")]
pub struct TipProofRequest {}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TipProofResponse {
    pub proof: RpcBlockProof,
}

#[derive(Clone, Copy, Debug, Serialize_tuple, Deserialize_tuple)]
#[serde(rename_all = "camelCase")]
pub struct TipKernelRequest {}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TipKernelResponse {
    pub kernel: RpcBlockKernel,
}

#[derive(Clone, Copy, Debug, Serialize_tuple, Deserialize_tuple)]
#[serde(rename_all = "camelCase")]
pub struct TipHeaderRequest {}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TipHeaderResponse {
    pub header: RpcBlockHeader,
}

#[derive(Clone, Copy, Debug, Serialize_tuple, Deserialize_tuple)]
#[serde(rename_all = "camelCase")]
pub struct TipBodyRequest {}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TipBodyResponse {
    pub body: RpcBlockBody,
}

#[derive(Clone, Copy, Debug, Serialize_tuple, Deserialize_tuple)]
#[serde(rename_all = "camelCase")]
pub struct TipTransactionKernelRequest {}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TipTransactionKernelResponse {
    pub kernel: RpcTransactionKernel,
}

#[derive(Clone, Copy, Debug, Serialize_tuple, Deserialize_tuple)]
#[serde(rename_all = "camelCase")]
pub struct TipAnnouncementsRequest {}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TipAnnouncementsResponse {
    pub announcements: Vec<RpcBFieldElements>,
}

#[derive(Clone, Copy, Debug, Serialize_tuple, Deserialize_tuple)]
#[serde(rename_all = "camelCase")]
pub struct GetBlockDigestRequest {
    pub selector: RpcBlockSelector,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBlockDigestResponse {
    pub digest: Option<Digest>,
}

#[derive(Clone, Copy, Debug, Serialize_tuple, Deserialize_tuple)]
#[serde(rename_all = "camelCase")]
pub struct GetBlockDigestsRequest {
    pub height: BFieldElement,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBlockDigestsResponse {
    pub digests: Vec<Digest>,
}

#[derive(Clone, Copy, Debug, Serialize_tuple, Deserialize_tuple)]
#[serde(rename_all = "camelCase")]
pub struct GetBlockRequest {
    pub selector: RpcBlockSelector,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBlockResponse {
    pub block: Option<RpcBlock>,
}

#[derive(Clone, Copy, Debug, Serialize_tuple, Deserialize_tuple)]
#[serde(rename_all = "camelCase")]
pub struct GetBlockProofRequest {
    pub selector: RpcBlockSelector,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBlockProofResponse {
    pub proof: Option<RpcBlockProof>,
}

#[derive(Clone, Copy, Debug, Serialize_tuple, Deserialize_tuple)]
#[serde(rename_all = "camelCase")]
pub struct GetBlockKernelRequest {
    pub selector: RpcBlockSelector,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBlockKernelResponse {
    pub kernel: Option<RpcBlockKernel>,
}

#[derive(Clone, Copy, Debug, Serialize_tuple, Deserialize_tuple)]
#[serde(rename_all = "camelCase")]
pub struct GetBlockHeaderRequest {
    pub selector: RpcBlockSelector,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBlockHeaderResponse {
    pub header: Option<RpcBlockHeader>,
}

#[derive(Clone, Copy, Debug, Serialize_tuple, Deserialize_tuple)]
#[serde(rename_all = "camelCase")]
pub struct GetBlockBodyRequest {
    pub selector: RpcBlockSelector,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBlockBodyResponse {
    pub body: Option<RpcBlockBody>,
}

#[derive(Clone, Copy, Debug, Serialize_tuple, Deserialize_tuple)]
#[serde(rename_all = "camelCase")]
pub struct GetBlockTransactionKernelRequest {
    pub selector: RpcBlockSelector,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBlockTransactionKernelResponse {
    pub kernel: Option<RpcTransactionKernel>,
}

#[derive(Clone, Copy, Debug, Serialize_tuple, Deserialize_tuple)]
#[serde(rename_all = "camelCase")]
pub struct GetBlockAnnouncementsRequest {
    pub selector: RpcBlockSelector,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBlockAnnouncementsResponse {
    pub announcements: Option<Vec<RpcBFieldElements>>,
}

#[derive(Clone, Copy, Debug, Serialize_tuple, Deserialize_tuple)]
#[serde(rename_all = "camelCase")]
pub struct IsBlockCanonicalRequest {
    pub digest: Digest,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IsBlockCanonicalResponse {
    pub canonical: bool,
}

#[derive(Clone, Copy, Debug, Serialize_tuple, Deserialize_tuple)]
#[serde(rename_all = "camelCase")]
pub struct GetUtxoDigestRequest {
    pub leaf_index: u64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetUtxoDigestResponse {
    pub digest: Option<Digest>,
}

#[derive(Clone, Copy, Debug, Serialize_tuple, Deserialize_tuple)]
#[serde(rename_all = "camelCase")]
pub struct FindUtxoOriginRequest {
    pub addition_record: RpcAdditionRecord,

    #[serde(default)]
    pub search_depth: Option<u64>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FindUtxoOriginResponse {
    pub block: Option<Digest>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetUtxosFromViewKeyRequest {
    pub view_key: String,
    pub start_block: u64,
    pub end_block: Option<u64>,
    pub max_search_depth: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetUtxosFromViewKeyResponse {
    pub utxos: Vec<IncomingUtxo>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBalanceFromViewKeyRequest {
    pub view_key: String,
    pub start_block: u64,
    pub end_block: Option<u64>,
    pub max_search_depth: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBalanceFromViewKeyResponse {
    pub balance: String,
    pub available_balance: String,
    pub pending_balance: String,
    pub utxo_count: u64,
    pub error: Option<String>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize_tuple)]
#[serde(rename_all = "camelCase")]
pub struct GetArchivalMutatorSetRequest {}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetArchivalMutatorSetResponse {
    pub archival_mutator_set: MutatorSetAccumulator,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateMembershipProofRequest {
    /// Hex-encoded bincode-serialized Tip5(item)
    pub item_hex: String,
    /// Hex-encoded bincode-serialized sender randomness
    pub sender_randomness_hex: String,
    /// Hex-encoded bincode-serialized receiver preimage
    pub receiver_preimage_hex: String,
    /// AOCL leaf index for the addition
    pub aocl_leaf_index: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateMembershipProofResponse {
    /// Hex-encoded bincode-serialized membership proof, if successful
    pub msmp_hex: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateUtxoWithProofRequest {
    /// Hex-encoded bincode-serialized UTXO
    pub utxo_hex: String,
    /// Hex-encoded bincode-serialized AdditionRecord (for locating AOCL index)
    pub addition_record_hex: String,
    /// Hex-encoded bincode-serialized sender randomness
    pub sender_randomness_hex: String,
    /// Hex-encoded bincode-serialized receiver preimage
    pub receiver_preimage_hex: String,
    /// Optional search bound
    pub max_search_depth: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateUtxoWithProofResponse {
    /// Hex-encoded bincode-serialized UTXO echoed back
    pub utxo_hex: Option<String>,
    /// Hex-encoded bincode-serialized membership proof
    pub msmp_hex: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetUtxoCreationBlockRequest {
    /// Hex-encoded bincode-serialized AdditionRecord identifying the UTXO
    pub addition_record: String,
    /// Maximum search depth for canonical chain lookup
    pub max_search_depth: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetUtxoCreationBlockResponse {
    /// Block height where this UTXO was created, if found
    pub last_aocl_index_in_block: Option<u64>,

    pub num_outputs_in_block: Option<u64>,
    /// Error description if lookup failed
    pub error: Option<String>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize_tuple)]
#[serde(rename_all = "camelCase")]
pub struct GetStateRequest {}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetStateResponse {
    /// Complete node state information
    pub state: StateInfo,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StateInfo {
    /// Current chain height
    pub height: BlockHeight,

    /// Current tip block hash
    pub tip_hash: String,

    /// Network name
    pub network: String,

    /// Wallet balance information
    pub wallet_balance: WalletBalanceInfo,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletBalanceInfo {
    pub confirmed_available: String,
    pub confirmed_total: String,
    pub unconfirmed_available: String,
    pub unconfirmed_total: String,
}

/// Request for submitting a pre-built transaction
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitTransactionRequest {
    /// Hex-encoded bincode-serialized Transaction
    pub transaction_hex: String,
}

/// Response for submitting a pre-built transaction
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitTransactionResponse {
    /// Transaction ID (hex)
    pub txid: String,

    /// Success status
    pub success: bool,

    /// Error message if failed
    pub error: Option<String>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize_tuple)]
#[serde(rename_all = "camelCase")]
pub struct GetCurrentFeeRequest {}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetCurrentFeeResponse {
    /// Minimum relay fee per input (from node configuration)
    pub min_relay_fee_per_input: String,
    /// Minimum fee from mempool transactions (if mempool has transactions)
    pub min_fee_from_mempool: Option<String>,
    /// Maximum fee from mempool transactions (if mempool has transactions)
    pub max_fee_from_mempool: Option<String>,
    /// Number of transactions in mempool used for fee calculation
    pub mempool_tx_count: u64,
    /// Error message if failed
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetUtxoInfoRequest {
    /// Hex-encoded bincode-serialized ViewKey (needed to decrypt UTXO from block)
    pub view_key: String,
    /// Start block height to scan from
    pub start_block: u64,
    /// End block height to scan to (optional, defaults to tip)
    pub end_block: Option<u64>,
    /// Maximum search depth
    pub max_search_depth: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UtxoInfo {
    /// Amount of the UTXO
    pub amount: String,
    /// Block height where the UTXO was created
    pub block_height: u64,
    /// Hex-encoded bincode-serialized AdditionRecord
    pub addition_record: String,
    /// Hex-encoded UTXO hash (Tip5 hash of the UTXO)
    pub utxo_hash: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetUtxoInfoResponse {
    /// List of UTXO information
    pub utxos: Vec<UtxoInfo>,
    /// Error message if failed
    pub error: Option<String>,
}

#[derive(Clone, Copy, Debug, Serialize_tuple, Deserialize_tuple)]
#[serde(rename_all = "camelCase")]
pub struct GetXntPriceRequest {}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetXntPriceResponse {
    pub price: String,
}
