#![allow(missing_docs)]

//! Execution-level correctness tests for mappings, events/logs, transient storage,
//! for-loop storage mutation, and checked arithmetic.
//!
//! Every test runs at O0, O1, O2, and O3 to catch optimizer bugs.

use revm::{
    context::{Context, TxEnv},
    database::{CacheDB, EmptyDB},
    handler::MainnetContext,
    primitives::{Address, Bytes, TxKind, U256},
    state::AccountInfo,
    ExecuteCommitEvm, MainBuilder, MainContext, MainnetEvm,
};
use tiny_keccak::{Hasher, Keccak};

use crate::helpers::{calldata, compile_contract_opt, decode_u256, encode_u256, selector, CALLER};

// =============================================================================
// Shared helpers (features-specific — EvmHandle returns CallResult with logs)
// =============================================================================

fn event_sig(sig: &str) -> [u8; 32] {
    let mut h = Keccak::v256();
    h.update(sig.as_bytes());
    let mut out = [0u8; 32];
    h.finalize(&mut out);
    out
}

const fn encode_addr(suffix: u8) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[31] = suffix;
    out
}

type TestDb = CacheDB<EmptyDB>;
type TestEvm = MainnetEvm<MainnetContext<TestDb>>;

struct EvmHandle {
    evm: TestEvm,
    contract: Address,
    nonce: u64,
}

impl EvmHandle {
    fn new(deploy_bytecode: Vec<u8>) -> Self {
        let mut db = CacheDB::<EmptyDB>::default();
        db.insert_account_info(
            CALLER,
            AccountInfo {
                balance: U256::from(u64::MAX),
                nonce: 0,
                ..Default::default()
            },
        );

        let mut evm = Context::mainnet().with_db(db).build_mainnet();

        let tx = TxEnv::builder()
            .caller(CALLER)
            .kind(TxKind::Create)
            .data(Bytes::from(deploy_bytecode))
            .gas_limit(10_000_000)
            .nonce(0)
            .build()
            .unwrap();

        let result = evm.transact_commit(tx).unwrap();
        assert!(result.is_success(), "Deployment failed: {result:#?}");

        let contract = CALLER.create(0);
        Self {
            evm,
            contract,
            nonce: 1,
        }
    }

    fn call(&mut self, calldata: Vec<u8>) -> CallResult {
        let tx = TxEnv::builder()
            .caller(CALLER)
            .kind(TxKind::Call(self.contract))
            .data(Bytes::from(calldata))
            .nonce(self.nonce)
            .gas_limit(10_000_000)
            .build()
            .unwrap();
        let result = self.evm.transact_commit(tx).unwrap();
        self.nonce += 1;
        let success = result.is_success();
        let output = result.output().map(|b| b.to_vec()).unwrap_or_default();
        let logs: Vec<LogEntry> = result
            .logs()
            .iter()
            .map(|l| LogEntry {
                topics: l.data.topics().iter().map(|t| t.0).collect(),
                data: l.data.data.to_vec(),
            })
            .collect();
        CallResult {
            success,
            output,
            logs,
        }
    }
}

#[derive(Debug)]
struct LogEntry {
    topics: Vec<[u8; 32]>,
    data: Vec<u8>,
}

#[derive(Debug)]
struct CallResult {
    success: bool,
    output: Vec<u8>,
    logs: Vec<LogEntry>,
}

fn for_all_opt_levels(contract_path: &str, test_fn: impl Fn(&mut EvmHandle, u8) + Sync) {
    std::thread::scope(|s| {
        let handles: Vec<_> = (0..=3)
            .map(|opt| {
                let test_fn = &test_fn;
                let path = contract_path;
                s.spawn(move || {
                    let bc = compile_contract_opt(path, opt);
                    let mut h = EvmHandle::new(bc);
                    test_fn(&mut h, opt);
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
    });
}

// =============================================================================
// Mapping correctness tests (examples/test_mappings.edge)
// =============================================================================

const MAPPINGS: &str = "examples/tests/test_mappings.edge";

#[test]
fn test_mapping_set_get() {
    for_all_opt_levels(MAPPINGS, |h, o| {
        let r = h.call(calldata(
            selector("map_set(address,uint256)"),
            &[encode_addr(0x42), encode_u256(100)],
        ));
        assert!(r.success, "map_set reverted at O{o}");
        let r = h.call(calldata(selector("map_get(address)"), &[encode_addr(0x42)]));
        assert!(r.success, "map_get reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 100, "map_get wrong at O{o}");
    });
}

#[test]
fn test_mapping_default_zero() {
    for_all_opt_levels(MAPPINGS, |h, o| {
        let r = h.call(calldata(selector("map_get(address)"), &[encode_addr(0xFF)]));
        assert!(r.success, "map_get reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 0, "unset key should be 0 at O{o}");
    });
}

#[test]
fn test_mapping_read_modify_write() {
    for_all_opt_levels(MAPPINGS, |h, o| {
        // Set initial value
        h.call(calldata(
            selector("map_set(address,uint256)"),
            &[encode_addr(0x01), encode_u256(50)],
        ));
        // Add 30
        h.call(calldata(
            selector("map_add(address,uint256)"),
            &[encode_addr(0x01), encode_u256(30)],
        ));
        let r = h.call(calldata(selector("map_get(address)"), &[encode_addr(0x01)]));
        assert!(r.success, "map_get reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 80, "50+30=80 at O{o}");
    });
}

#[test]
fn test_mapping_two_keys_independent() {
    for_all_opt_levels(MAPPINGS, |h, o| {
        h.call(calldata(
            selector("two_keys(address,address,uint256,uint256)"),
            &[
                encode_addr(0x01),
                encode_addr(0x02),
                encode_u256(111),
                encode_u256(222),
            ],
        ));
        let r1 = h.call(calldata(selector("map_get(address)"), &[encode_addr(0x01)]));
        let r2 = h.call(calldata(selector("map_get(address)"), &[encode_addr(0x02)]));
        assert_eq!(decode_u256(&r1.output), 111, "key1 wrong at O{o}");
        assert_eq!(decode_u256(&r2.output), 222, "key2 wrong at O{o}");
    });
}

#[test]
fn test_nested_mapping_set_get() {
    for_all_opt_levels(MAPPINGS, |h, o| {
        h.call(calldata(
            selector("nested_set(address,address,uint256)"),
            &[encode_addr(0x01), encode_addr(0x02), encode_u256(999)],
        ));
        let r = h.call(calldata(
            selector("nested_get(address,address)"),
            &[encode_addr(0x01), encode_addr(0x02)],
        ));
        assert!(r.success, "nested_get reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 999, "nested_get wrong at O{o}");
    });
}

#[test]
fn test_nested_mapping_different_keys() {
    for_all_opt_levels(MAPPINGS, |h, o| {
        // allowances[0x01][0x02] = 100, allowances[0x01][0x03] = 200
        h.call(calldata(
            selector("nested_two_spenders(address,address,address,uint256,uint256)"),
            &[
                encode_addr(0x01),
                encode_addr(0x02),
                encode_addr(0x03),
                encode_u256(100),
                encode_u256(200),
            ],
        ));
        let r1 = h.call(calldata(
            selector("nested_get(address,address)"),
            &[encode_addr(0x01), encode_addr(0x02)],
        ));
        let r2 = h.call(calldata(
            selector("nested_get(address,address)"),
            &[encode_addr(0x01), encode_addr(0x03)],
        ));
        assert_eq!(decode_u256(&r1.output), 100, "spender1 wrong at O{o}");
        assert_eq!(decode_u256(&r2.output), 200, "spender2 wrong at O{o}");
    });
}

#[test]
fn test_mapping_counter_increment() {
    for_all_opt_levels(MAPPINGS, |h, o| {
        for _ in 0..5 {
            h.call(calldata(
                selector("counter_inc(address)"),
                &[encode_addr(0xAA)],
            ));
        }
        let r = h.call(calldata(
            selector("counter_get(address)"),
            &[encode_addr(0xAA)],
        ));
        assert_eq!(decode_u256(&r.output), 5, "counter should be 5 at O{o}");
    });
}

// =============================================================================
// Event/log correctness tests (examples/test_logs.edge)
// =============================================================================

const LOGS: &str = "examples/tests/test_logs.edge";

#[test]
fn test_log_no_indexed() {
    for_all_opt_levels(LOGS, |h, o| {
        let r = h.call(calldata(
            selector("emit_no_indexed(uint256)"),
            &[encode_u256(42)],
        ));
        assert!(r.success, "emit_no_indexed reverted at O{o}");
        assert_eq!(
            r.logs.len(),
            1,
            "expected 1 log at O{o}, got {}",
            r.logs.len()
        );

        let log = &r.logs[0];
        // topic0 = keccak256("NoIndexed(uint256)")
        let expected_sig = event_sig("NoIndexed(uint256)");
        assert_eq!(log.topics.len(), 1, "expected 1 topic at O{o}");
        assert_eq!(log.topics[0], expected_sig, "wrong event sig at O{o}");
        // data = ABI-encoded value (42)
        assert_eq!(log.data.len(), 32, "data should be 32 bytes at O{o}");
        assert_eq!(
            &log.data[24..32],
            &42u64.to_be_bytes(),
            "data wrong at O{o}"
        );

        // Verify marker
        let m = h.call(calldata(selector("get_marker()"), &[]));
        assert_eq!(decode_u256(&m.output), 1, "marker wrong at O{o}");
    });
}

#[test]
fn test_log_one_indexed() {
    for_all_opt_levels(LOGS, |h, o| {
        let r = h.call(calldata(
            selector("emit_one_indexed(uint256,uint256)"),
            &[encode_u256(7), encode_u256(99)],
        ));
        assert!(r.success, "emit_one_indexed reverted at O{o}");
        assert_eq!(r.logs.len(), 1, "expected 1 log at O{o}");

        let log = &r.logs[0];
        assert_eq!(log.topics.len(), 2, "expected 2 topics at O{o}");
        // topic0 = event sig
        assert_eq!(
            log.topics[0],
            event_sig("OneIndexed(uint256,uint256)"),
            "wrong sig at O{o}"
        );
        // topic1 = indexed key = 7
        assert_eq!(
            &log.topics[1][24..32],
            &7u64.to_be_bytes(),
            "indexed key wrong at O{o}"
        );
        // data = non-indexed value = 99
        assert_eq!(
            &log.data[24..32],
            &99u64.to_be_bytes(),
            "data wrong at O{o}"
        );
    });
}

#[test]
fn test_log_two_indexed() {
    for_all_opt_levels(LOGS, |h, o| {
        let r = h.call(calldata(
            selector("emit_two_indexed(address,address,uint256)"),
            &[encode_addr(0x01), encode_addr(0x02), encode_u256(500)],
        ));
        assert!(r.success, "emit_two_indexed reverted at O{o}");
        assert_eq!(r.logs.len(), 1, "expected 1 log at O{o}");

        let log = &r.logs[0];
        assert_eq!(log.topics.len(), 3, "expected 3 topics at O{o}");
        assert_eq!(
            log.topics[0],
            event_sig("TwoIndexed(address,address,uint256)"),
            "wrong sig at O{o}"
        );
        // topic1 = indexed from = addr(0x01)
        assert_eq!(log.topics[1][31], 0x01, "from wrong at O{o}");
        // topic2 = indexed to = addr(0x02)
        assert_eq!(log.topics[2][31], 0x02, "to wrong at O{o}");
        // data = amount = 500
        assert_eq!(
            &log.data[24..32],
            &500u64.to_be_bytes(),
            "amount wrong at O{o}"
        );
    });
}

#[test]
fn test_log_three_indexed() {
    for_all_opt_levels(LOGS, |h, o| {
        let r = h.call(calldata(
            selector("emit_three_indexed(uint256,uint256,uint256)"),
            &[encode_u256(10), encode_u256(20), encode_u256(30)],
        ));
        assert!(r.success, "emit_three_indexed reverted at O{o}");
        assert_eq!(r.logs.len(), 1, "expected 1 log at O{o}");

        let log = &r.logs[0];
        assert_eq!(log.topics.len(), 4, "expected 4 topics at O{o}");
        assert_eq!(
            log.topics[0],
            event_sig("ThreeIndexed(uint256,uint256,uint256)"),
            "wrong sig at O{o}"
        );
        assert_eq!(
            &log.topics[1][24..32],
            &10u64.to_be_bytes(),
            "a wrong at O{o}"
        );
        assert_eq!(
            &log.topics[2][24..32],
            &20u64.to_be_bytes(),
            "b wrong at O{o}"
        );
        assert_eq!(
            &log.topics[3][24..32],
            &30u64.to_be_bytes(),
            "c wrong at O{o}"
        );
        // No non-indexed data for ThreeIndexed
        assert_eq!(log.data.len(), 0, "should have no data at O{o}");
    });
}

// =============================================================================
// Transient storage correctness tests (examples/test_transient.edge)
// =============================================================================

const TRANSIENT: &str = "examples/tests/test_transient.edge";

#[test]
fn test_transient_set_get_within_tx() {
    for_all_opt_levels(TRANSIENT, |h, o| {
        // Set tval=42
        let r = h.call(calldata(selector("set_tval(uint256)"), &[encode_u256(42)]));
        assert!(r.success, "set_tval reverted at O{o}");

        // Get tval — should be 0 because transient storage clears between transactions
        let r = h.call(calldata(selector("get_tval()"), &[]));
        assert!(r.success, "get_tval reverted at O{o}");
        assert_eq!(
            decode_u256(&r.output),
            0,
            "transient should clear between txs at O{o}"
        );
    });
}

#[test]
fn test_transient_clears_between_txs() {
    for_all_opt_levels(TRANSIENT, |h, o| {
        // Set and immediately read in separate txs
        h.call(calldata(selector("set_tval(uint256)"), &[encode_u256(99)]));
        // New tx — transient should be cleared
        let r = h.call(calldata(selector("get_tval()"), &[]));
        assert_eq!(decode_u256(&r.output), 0, "transient not cleared at O{o}");
    });
}

#[test]
fn test_transient_persistent_independent() {
    for_all_opt_levels(TRANSIENT, |h, o| {
        // set_both: tval=77, counter=10
        h.call(calldata(
            selector("set_both(uint256,uint256)"),
            &[encode_u256(77), encode_u256(10)],
        ));

        // In next tx: tval should be 0 (transient), counter should be 10 (persistent)
        let r_t = h.call(calldata(selector("get_tval()"), &[]));
        assert_eq!(decode_u256(&r_t.output), 0, "transient should be 0 at O{o}");
        let r_c = h.call(calldata(selector("get_counter()"), &[]));
        assert_eq!(
            decode_u256(&r_c.output),
            10,
            "persistent counter should be 10 at O{o}"
        );
    });
}

#[test]
fn test_transient_two_fields_independent() {
    for_all_opt_levels(TRANSIENT, |h, o| {
        // tval and tval2 use different transient slots
        h.call(calldata(selector("set_tval(uint256)"), &[encode_u256(11)]));
        h.call(calldata(selector("set_tval2(uint256)"), &[encode_u256(22)]));
        // Both should be 0 in next tx (cleared)
        let r1 = h.call(calldata(selector("get_tval()"), &[]));
        let r2 = h.call(calldata(selector("get_tval2()"), &[]));
        assert_eq!(decode_u256(&r1.output), 0, "tval not cleared at O{o}");
        assert_eq!(decode_u256(&r2.output), 0, "tval2 not cleared at O{o}");
    });
}

#[test]
fn test_transient_persistent_survives() {
    for_all_opt_levels(TRANSIENT, |h, o| {
        // Increment counter 3 times
        for _ in 0..3 {
            h.call(calldata(selector("inc_counter()"), &[]));
        }
        let r = h.call(calldata(selector("get_counter()"), &[]));
        assert_eq!(
            decode_u256(&r.output),
            3,
            "persistent counter should survive at O{o}"
        );
    });
}

// =============================================================================
// For-loop storage mutation tests (examples/test_loop_storage.edge)
// =============================================================================

const LOOP_STORAGE: &str = "examples/tests/test_loop_storage.edge";

#[test]
fn test_loop_accumulate() {
    for_all_opt_levels(LOOP_STORAGE, |h, o| {
        // accumulate(5): total += 0 + 1 + 2 + 3 + 4 = 10
        h.call(calldata(selector("accumulate(uint256)"), &[encode_u256(5)]));
        let r = h.call(calldata(selector("get_total()"), &[]));
        assert_eq!(decode_u256(&r.output), 10, "accumulate(5) wrong at O{o}");
    });
}

#[test]
fn test_loop_accumulate_zero() {
    for_all_opt_levels(LOOP_STORAGE, |h, o| {
        // accumulate(0): loop doesn't execute
        h.call(calldata(selector("accumulate(uint256)"), &[encode_u256(0)]));
        let r = h.call(calldata(selector("get_total()"), &[]));
        assert_eq!(decode_u256(&r.output), 0, "accumulate(0) wrong at O{o}");
    });
}

#[test]
fn test_loop_count_up() {
    for_all_opt_levels(LOOP_STORAGE, |h, o| {
        // count_up(7): count = 7
        h.call(calldata(selector("count_up(uint256)"), &[encode_u256(7)]));
        let r = h.call(calldata(selector("get_count()"), &[]));
        assert_eq!(decode_u256(&r.output), 7, "count_up(7) wrong at O{o}");
    });
}

#[test]
fn test_loop_read_write_same_iteration() {
    for_all_opt_levels(LOOP_STORAGE, |h, o| {
        // read_write_loop(3):
        //   i=0: prev=0, total=0+0+1=1, last_val=0
        //   i=1: prev=1, total=1+1+1=3, last_val=1
        //   i=2: prev=3, total=3+2+1=6, last_val=3
        h.call(calldata(
            selector("read_write_loop(uint256)"),
            &[encode_u256(3)],
        ));
        let r_total = h.call(calldata(selector("get_total()"), &[]));
        assert_eq!(decode_u256(&r_total.output), 6, "total wrong at O{o}");
        let r_last = h.call(calldata(selector("get_last_val()"), &[]));
        assert_eq!(decode_u256(&r_last.output), 3, "last_val wrong at O{o}");
    });
}

#[test]
fn test_loop_multiple_calls_accumulate() {
    for_all_opt_levels(LOOP_STORAGE, |h, o| {
        // accumulate(3): total = 0+1+2 = 3
        h.call(calldata(selector("accumulate(uint256)"), &[encode_u256(3)]));
        // accumulate(3) again: total = 3 + 0+1+2 = 6
        h.call(calldata(selector("accumulate(uint256)"), &[encode_u256(3)]));
        let r = h.call(calldata(selector("get_total()"), &[]));
        assert_eq!(decode_u256(&r.output), 6, "double accumulate wrong at O{o}");
    });
}

#[test]
fn test_loop_reset_and_reaccumulate() {
    for_all_opt_levels(LOOP_STORAGE, |h, o| {
        // Accumulate, reset, accumulate again
        h.call(calldata(
            selector("accumulate(uint256)"),
            &[encode_u256(10)],
        ));
        h.call(calldata(selector("reset()"), &[]));
        let r = h.call(calldata(selector("get_total()"), &[]));
        assert_eq!(decode_u256(&r.output), 0, "reset didn't clear at O{o}");

        h.call(calldata(selector("accumulate(uint256)"), &[encode_u256(4)]));
        let r = h.call(calldata(selector("get_total()"), &[]));
        assert_eq!(decode_u256(&r.output), 6, "re-accumulate(4) wrong at O{o}");
    });
}

// =============================================================================
// Checked arithmetic correctness tests (examples/test_checked_arith.edge)
// =============================================================================

const CHECKED: &str = "examples/tests/test_checked_arith.edge";

#[test]
fn test_checked_add_safe() {
    for_all_opt_levels(CHECKED, |h, o| {
        let r = h.call(calldata(
            selector("safe_add(uint256,uint256)"),
            &[encode_u256(100), encode_u256(200)],
        ));
        assert!(r.success, "safe_add(100,200) reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 300, "100+200=300 at O{o}");
    });
}

#[test]
fn test_checked_add_overflow_reverts() {
    for_all_opt_levels(CHECKED, |h, o| {
        // max u256 + 1 should overflow and revert
        let max_u256 = [0xFFu8; 32];
        let r = h.call(calldata(
            selector("safe_add(uint256,uint256)"),
            &[max_u256, encode_u256(1)],
        ));
        assert!(!r.success, "safe_add(MAX, 1) should revert at O{o}");
    });
}

#[test]
fn test_checked_sub_safe() {
    for_all_opt_levels(CHECKED, |h, o| {
        let r = h.call(calldata(
            selector("safe_sub(uint256,uint256)"),
            &[encode_u256(500), encode_u256(200)],
        ));
        assert!(r.success, "safe_sub(500,200) reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 300, "500-200=300 at O{o}");
    });
}

#[test]
fn test_checked_sub_underflow_reverts() {
    for_all_opt_levels(CHECKED, |h, o| {
        // 5 - 10 should underflow and revert
        let r = h.call(calldata(
            selector("safe_sub(uint256,uint256)"),
            &[encode_u256(5), encode_u256(10)],
        ));
        assert!(!r.success, "safe_sub(5, 10) should revert at O{o}");
    });
}

#[test]
fn test_checked_mul_safe() {
    for_all_opt_levels(CHECKED, |h, o| {
        let r = h.call(calldata(
            selector("safe_mul(uint256,uint256)"),
            &[encode_u256(7), encode_u256(8)],
        ));
        assert!(r.success, "safe_mul(7,8) reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 56, "7*8=56 at O{o}");
    });
}

#[test]
fn test_checked_mul_overflow_reverts() {
    for_all_opt_levels(CHECKED, |h, o| {
        // max_u256 * 2 should overflow
        let max_u256 = [0xFFu8; 32];
        let r = h.call(calldata(
            selector("safe_mul(uint256,uint256)"),
            &[max_u256, encode_u256(2)],
        ));
        assert!(!r.success, "safe_mul(MAX, 2) should revert at O{o}");
    });
}

#[test]
fn test_checked_masked_add_elided() {
    for_all_opt_levels(CHECKED, |h, o| {
        // (x & 255) + 1 — should never overflow even with max input
        let max_u256 = [0xFFu8; 32];
        let r = h.call(calldata(selector("masked_add(uint256)"), &[max_u256]));
        assert!(r.success, "masked_add should not revert at O{o}");
        assert_eq!(decode_u256(&r.output), 256, "(0xFF & 255)+1=256 at O{o}");
    });
}

#[test]
fn test_checked_chain_safe() {
    for_all_opt_levels(CHECKED, |h, o| {
        // chain_safe(10, 20): sum=30, doubled=60, result=60-10=50
        let r = h.call(calldata(
            selector("chain_safe(uint256,uint256)"),
            &[encode_u256(10), encode_u256(20)],
        ));
        assert!(r.success, "chain_safe(10,20) reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 50, "chain_safe wrong at O{o}");
    });
}

#[test]
fn test_checked_sub_zero() {
    for_all_opt_levels(CHECKED, |h, o| {
        // 0 - 0 = 0, should not underflow
        let r = h.call(calldata(
            selector("safe_sub(uint256,uint256)"),
            &[encode_u256(0), encode_u256(0)],
        ));
        assert!(r.success, "safe_sub(0,0) reverted at O{o}");
        assert_eq!(decode_u256(&r.output), 0, "0-0=0 at O{o}");
    });
}

#[test]
fn test_checked_mul_zero() {
    for_all_opt_levels(CHECKED, |h, o| {
        // max * 0 = 0, should not overflow
        let max_u256 = [0xFFu8; 32];
        let r = h.call(calldata(
            selector("safe_mul(uint256,uint256)"),
            &[max_u256, encode_u256(0)],
        ));
        assert!(r.success, "safe_mul(MAX, 0) should not revert at O{o}");
        assert_eq!(decode_u256(&r.output), 0, "MAX*0=0 at O{o}");
    });
}
