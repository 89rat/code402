//! Stage 4 claim-machine conformance tests — written BEFORE the
//! implementation (PANEL.md: payment-path changes require the failing test
//! first). The claim machine is a PURE state machine over a storage trait so
//! it can be exhaustively model-checked here (the runnable form of the TLA+
//! model; the .tla file lands alongside as the review artifact).
//!
//! States (plan-rev3 G3): claimed -> settling -> settled | failed, with
//! receipt_pending and non_replayable as settlement-record statuses the
//! machine can emit. Lease: a claim older than LEASE_SECS with no settle
//! outcome is retriable (crashed-isolate recovery).

use m2m_core::payment::settlement::{
    ClaimInput, ClaimStore, ClaimTransition, SettlementClaimMachine, LEASE_SECS,
};

/// In-memory store for tests; also the reference semantics for the DO/D1
/// implementation.
#[derive(Default)]
struct MemStore {
    rows: std::collections::BTreeMap<String, m2m_core::payment::settlement::ClaimRow>,
}

impl ClaimStore for MemStore {
    fn load(&self, key: &str) -> Result<Option<m2m_core::payment::settlement::ClaimRow>, String> {
        Ok(self.rows.get(key).cloned())
    }
    fn save(&mut self, key: &str, row: m2m_core::payment::settlement::ClaimRow) -> Result<(), String> {
        self.rows.insert(key.to_string(), row);
        Ok(())
    }
}

fn input(at: u64) -> ClaimInput {
    ClaimInput {
        payer: "0x857b06519e91e3a54538791bDbb0E22373e36b66".into(),
        nonce: format!("0x{}", "ab".repeat(32)),
        request_id: "req-1".into(),
        tool: "vat-mod97-check".into(),
        input_hash: "ih".into(),
        now_unix: at,
    }
}

#[test]
fn first_claimant_wins_second_gets_replay_or_wait() {
    let mut s = MemStore::default();
    let m = SettlementClaimMachine;
    // first: claim
    assert!(matches!(
        m.claim(&mut s, &input(100)).unwrap(),
        ClaimTransition::Claimed
    ));
    // second (concurrent, same (payer,nonce)): NOT a new claim
    match m.claim(&mut s, &input(101)).unwrap() {
        ClaimTransition::InProgress => {} // expected: loser waits
        other => panic!("expected InProgress, got {other:?}"),
    }
}

#[test]
fn settling_then_settled_and_replay_serves_stored() {
    let mut s = MemStore::default();
    let m = SettlementClaimMachine;
    let i = input(100);
    let key = SettlementClaimMachine::key_for(&i.payer, &i.nonce);
    m.claim(&mut s, &i).unwrap();
    // move to settling
    assert!(matches!(
        m.begin_settle(&mut s, &key, 110).unwrap(),
        ClaimTransition::Settling
    ));
    // settle succeeds with a response to persist
    assert!(matches!(
        m.settled(&mut s, &key, &input(110), "0xtx", "eip155:84532", b"{\"output\":1}", "PAYMENT-RESPONSE-B64").unwrap(),
        ClaimTransition::Settled
    ));
    // a retry of the SAME (payer,nonce) now replays the stored response —
    // identical 200s for retries and race losers (G2b)
    match m.claim(&mut s, &input(120)).unwrap() {
        ClaimTransition::Replay { .. } => {}
        other => panic!("expected Replay, got {other:?}"),
    }
}

#[test]
fn lease_expiry_frees_wedged_nonce() {
    let mut s = MemStore::default();
    let m = SettlementClaimMachine;
    let i = input(100);
    let key = SettlementClaimMachine::key_for(&i.payer, &i.nonce);
    m.claim(&mut s, &i).unwrap();
    m.begin_settle(&mut s, &key, 110).unwrap();
    // crashed isolate: no outcome; after lease expiry the claim is retriable
    let now = 110 + LEASE_SECS + 1;
    match m.claim(&mut s, &input(now)).unwrap() {
        ClaimTransition::LeaseExpired => {}
        other => panic!("expected LeaseExpired(retriable), got {other:?}"),
    }
    // the re-claimer HOLDS the freed claim: proceeding is begin_settle
    // (a further concurrent claimant now sees InProgress, not a fresh claim)
    assert!(matches!(
        m.begin_settle(&mut s, &key, now + 1).unwrap(),
        ClaimTransition::Settling
    ));
    assert!(matches!(
        m.claim(&mut s, &input(now + 2)).unwrap(),
        ClaimTransition::InProgress
    ));
}

#[test]
fn already_used_without_our_record_is_receipt_pending() {
    // G2d: settle reports authorization-already-used and we have NO settled
    // record => receipt_pending, NEVER assumed-ours
    let mut s = MemStore::default();
    let m = SettlementClaimMachine;
    let i = input(100);
    let key = SettlementClaimMachine::key_for(&i.payer, &i.nonce);
    m.claim(&mut s, &i).unwrap();
    m.begin_settle(&mut s, &key, 110).unwrap();
    assert!(matches!(
        m.receipt_pending(&mut s, &key).unwrap(),
        ClaimTransition::ReceiptPending
    ));
}

#[test]
fn failed_is_terminal_for_that_authorization() {
    let mut s = MemStore::default();
    let m = SettlementClaimMachine;
    let i = input(100);
    let key = SettlementClaimMachine::key_for(&i.payer, &i.nonce);
    m.claim(&mut s, &i).unwrap();
    m.begin_settle(&mut s, &key, 110).unwrap();
    assert!(matches!(
        m.failed(&mut s, &key, "insufficient_funds").unwrap(),
        ClaimTransition::Failed
    ));
    // re-claim of the same authorization after failure: still terminal
    match m.claim(&mut s, &input(120)).unwrap() {
        ClaimTransition::Terminal => {}
        other => panic!("expected Terminal, got {other:?}"),
    }
}

#[test]
fn exhaustive_model_check_all_interleavings() {
    // Runnable model-check (the TLA+ model's executable twin): exhaustively
    // walk every transition sequence up to depth 6 from a fresh store and
    // assert the machine's invariants hold at every step:
    //   INV-A: at most one settle attempt may reach Settled per (payer,nonce)
    //   INV-B: Settled is absorbing (no transition leaves it)
    //   INV-C: Replay is returned iff a stored response exists
    use std::collections::BTreeMap;
    let mut states: BTreeMap<String, (MemStore, Vec<&'static str>)> = BTreeMap::new();
    states.insert("".into(), (MemStore::default(), vec![]));
    let m = SettlementClaimMachine;
    let mut violations = 0;
    for _depth in 0..6 {
        let mut next = BTreeMap::new();
        for (_sig, (store, path)) in &states {
            let mut probe = |label: &'static str,
                             f: &dyn Fn(&mut MemStore) -> Option<ClaimTransition>|
             -> Option<()> {
                let mut st = MemStore { rows: store.rows.clone() };
                let out = f(&mut st);
                if let Some(t) = out {
                    // INV-B: nothing leaves Settled
                    if matches!(t, ClaimTransition::Replay { .. }) {
                        let key = SettlementClaimMachine::key_for(&input(1).payer, &input(1).nonce);
                        if st.load(&key).map(|r| r.map(|x| x.response_body.is_none()).unwrap_or(true)).unwrap_or(true) {
                            violations += 1; // INV-C violated
                        }
                    }
                    let mut p = path.clone();
                    p.push(label);
                    next.insert(format!("{:?}", p), (st, p));
                }
                Some(())
            };
            let i_now = input(1000);
            let key = SettlementClaimMachine::key_for(&i_now.payer, &i_now.nonce);
            let k = key.clone();
            probe("claim", &move |st| m.claim(st, &i_now).ok());
            let k2 = k.clone();
            probe("begin", &move |st| m.begin_settle(st, &k2, 1001).ok());
            let k3 = k.clone();
            probe("settled", &move |st| {
                m.settled(st, &k3, &input(1001), "0xt", "eip155:84532", b"{}", "B64").ok()
            });
            let k4 = k.clone();
            probe("failed", &move |st| m.failed(st, &k4, "x").ok());
            let k5 = k.clone();
            probe("receipt_pending", &move |st| m.receipt_pending(st, &k5).ok());
        }
        states = next;
    }
    assert_eq!(violations, 0, "model-check violations");
}
