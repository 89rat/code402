## Breaks

### Break 1 — I5: malformed/missing `amount_minor` fails open to 5000 minor units

**Invariant targeted:** I5 — fail closed on money.

**Exact input:**
1. Seed the local `PRICING` KV record for tool `vat-mod97-check` with malformed money config:
   ```json
   {}
   ```
   or with a string amount:
   ```json
   { "amount_minor": "100" }
   ```
2. Ensure `ops:x402v2_enabled=true` in the same KV.
3. Send:
   ```http
   POST /v2/tools/vat-mod97-check/call
   Content-Type: application/json

   {"input":{"vat_number":"GB123456789"}}
   ```
   with no `PAYMENT-SIGNATURE`.

**Defense fails at:** `crates/edge/src/x402v2_route.rs:141`

```rust
let amount = price["amount_minor"].as_u64().unwrap_or(5000);
```

`as_u64()` returns `None` for missing, null, string, or fractional money config; the route silently substitutes `5000` and issues a 402 challenge for 5000 minor units instead of failing closed.

**Fixture to make this permanent:**

Add an e2e/dev test, e.g. in `tests/v2-wire/run.sh` or a Rust integration fixture, that seeds:

```json
{}
```

for the tool, POSTs the same body, and asserts:

- status is in `5xx` or no `PAYMENT-REQUIRED` header is returned; and
- specifically no 402 challenge with `accepts[0].amount == "5000"` is issued.

Code fix to accompany the fixture: replace `unwrap_or(5000)` with an explicit `ok_or_else(...)?` error path and require `amount > 0` for money safety.

---

### Break 2 — I2: `SelectionPolicy` has no approved-payee gate, so a malicious 402 can direct spend to an arbitrary recipient

**Invariant targeted:** I2 — no spend without policy.

**Exact input:**
1. Crawler receives a crafted 402 header containing:
   ```json
   {
     "x402Version": 2,
     "resource": {"url": "https://attacker.invalid/call"},
     "accepts": [
       {
         "scheme": "exact",
         "network": "eip155:84532",
         "amount": "100",
         "asset": "0xAllowedUsdc",
         "pay_to": "0xAttackerWallet",
         "max_timeout_seconds": 300,
         "extra": {}
       }
     ],
     "extensions": {
       "code402.stamp": {
         "info": {"mac": "0x...", "iat": 1700000000},
         "schema": {}
       }
     }
   }
   ```
2. Client policy:
   ```rust
   SelectionPolicy {
       allowed_networks: vec!["eip155:84532".into()],
       allowed_assets: vec!["0xAllowedUsdc".into()],
       max_amount: U256::from(1000),
   }
   ```
3. Call:
   ```rust
   let selected = policy.select(&pr)?;
   let auth = build_authorization_from(selected, signer); // pay_to = 0xAttackerWallet
   sign_payment(selected, &auth, Signer::Eoa(sk), pr.extensions)?;
   ```

**Defense fails at:** `crates/core/src/payment/x402v2_client.rs:95-104`

```rust
if !self.allowed_networks.contains(&a.network) {
    continue;
}
if !self.allowed_assets.iter().any(|x| x.eq_ignore_ascii_case(&a.asset)) {
    continue;
}
if a.amount_u256()? > self.max_amount {
    continue;
}
a.validate_spec()?;
return Ok(a);
```

The loop checks only network, asset, and amount. It never checks `pay_to`, so any policy-allowed network/asset/amount combination can be routed to `0xAttackerWallet`.

**Fixture to make this permanent:**

Add a unit test:

```rust
#[test]
fn selection_denies_unapproved_payee() {
    let policy = SelectionPolicy {
        allowed_networks: vec!["eip155:84532".into()],
        allowed_assets: vec!["0xAllowedUsdc".into()],
        max_amount: U256::from(1_000),
    };

    let pr = /* PaymentRequired with pay_to = 0xAttackerWallet */;
    assert!(matches!(policy.select(&pr), Err(X402Error::BadNetwork(_))));
}
```

The current code fails this test because `select` returns `Ok`.

The code fix is to add an `allowed_payees` field to `SelectionPolicy` and continue/skip when the requirement `pay_to` is not in that list.

---

### Break 3 — I2: `sign_payment` signs an arbitrary authorization without binding it to a policy-selected requirement

**Invariant targeted:** I2 — no spend without policy.

**Exact input:**
1. Take a requirement that would pass policy:
   ```rust
   let requirement = PaymentRequirements {
       scheme: "exact".into(),
       network: "eip155:84532".into(),
       amount: "100".into(),
       asset: "0xAllowedUsdc".into(),
       pay_to: "0xApprovedWallet".into(),
       max_timeout_seconds: 300,
       extra: None,
   };
   ```
2. Build an authorization that does **not** match the policy cap:
   ```rust
   let auth = build_authorization(&AuthorizationParams {
       payer: payer_addr,
       pay_to: attacker_addr,          // arbitrary
       value: U256::from(10_000),      // actual spend exceeds policy max
       nonce: [0u8; 32],
       valid_after_unix: now,
       valid_before_unix: now + 300,
   });
   ```
3. Call:
   ```rust
   sign_payment(&requirement, &auth, Signer::Eoa(sk), None)
   ```

**Defense fails at:** `crates/core/src/payment/x402v2_client.rs:168-193`

The `Signer::Eoa` branch signs the payload directly. There is no `SelectionPolicy` argument and no assertion that the signed `auth.value` equals the policy-selected `requirement.amount_u256()?` or that `auth.to` equals the policy-approved payee.

Result: `SignedPayment::Signed { payload, b64 }` is returned for an authorization whose actual spend may exceed policy or go to an unapproved payee.

**Fixture to make this permanent:**

Add a unit test:

```rust
#[test]
fn sign_payment_refuses_authorization_not_matching_requirement() {
    let requirement = /* amount = 100 */;
    let auth = build_authorization(&AuthorizationParams {
        value: U256::from(10_000),
        pay_to: attacker_addr,
        // ...
    });

    assert!(sign_payment(&requirement, &auth, Signer::Eoa(sk), None).is_err());
}
```

Current code fails this test because it signs successfully.

Code fix: require either a `SelectionPolicy` inside `sign_payment`, or require `auth.value == requirement.amount_u256()?` and `auth.to == requirement.pay_to_addr()?` before signing.

---

### Break 4 — I5: invalid `CHAIN_ID` silently defaults to Base mainnet

**Invariant targeted:** I5 — fail closed on money.

**Exact input:**
1. Configure:
   ```dotenv
   CHAIN_ID=not-a-number
   ```
2. Send the same payment-free POST as in Break 1.

**Defense fails at:** `crates/edge/src/x402v2_route.rs:215`

```rust
let chain_id: u64 = env.var("CHAIN_ID")?.to_string().parse().unwrap_or(8453);
```

When `CHAIN_ID` is present but invalid, the route silently issues a 402 challenge for `eip155:8453` instead of failing closed.

**Fixture to make this permanent:**

Add an integration test that sets `CHAIN_ID` to an invalid string and asserts the route returns `5xx` or does not emit a 402 with a default `eip155:8453` requirement.

Code fix: parse `CHAIN_ID` with an explicit error path rather than `unwrap_or`.

---

## Holds

- **I1:** `x402v2_route.rs:210` — valid payment in dev serves without settlement; known accepted Stage 3/Stage 4 gap, route dark in prod.
- **I3:** `x402v2_route.rs:173-184` — replay of the same signed payload within the stamp grace window is not blocked by a merchant nonce ledger; known accepted.
- **I4:** `x402v2_client.rs:206-209` — `parse_settle_response` only performs schema/decode validation, not chain reconciliation; known accepted Stage 4/C2 gap.
- **I6:** `x402v2_route.rs:141` and `x402v2_client.rs:95-104` — body/content does not feed payment selection or amount; it holds in the current code.