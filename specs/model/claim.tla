------------------------------- MODULE claim -------------------------------
(* Stage 4 claim machine (G3) — TLA+ review artifact. The executable twin is
   crates/core/tests/settlement_claim.rs::exhaustive_model_check_all_interleavings
   (runnable in CI). States map 1:1 to ClaimStatus; lease maps to LEASE_SECS. *)

CONSTANT LeaseSecs

VARIABLES status, claimedAt

States == { "claimed", "settling", "settled", "failed", "receipt_pending" }

Init == /\ status \in { "claimed" }
        /\ claimedAt \in Nat

(* a holder's lease expires: any claimant may re-claim *)
LeaseExpires(at) == /\ status \in { "claimed", "settling" }
                    /\ at - claimedAt > LeaseSecs
                    /\ status' = "claimed"
                    /\ claimedAt' = at

BeginSettle(at) == /\ status = "claimed"
                   /\ status' = "settling"
                   /\ claimedAt' = at

Settled == /\ status \in { "claimed", "settling" }
           /\ status' = "settled"          \* absorbing

Failed == /\ status \in { "claimed", "settling" }
          /\ status' = "failed"            \* absorbing

ReceiptPending == /\ status \in { "claimed", "settling" }
                  /\ status' = "receipt_pending"  \* absorbing until cron

Next == \E at \in Nat : LeaseExpires(at) \/ BeginSettle(at)
        \/ Settled \/ Failed \/ ReceiptPending

(* INV-A: settled is reached at most once per (from, nonce) instance *)
(* INV-B: settled/failed/receipt_pending are absorbing *)
Absorbing == ~ /\ status \in { "settled", "failed", "receipt_pending" }
               /\ status' # status

=============================================================================
