import json, io, re, sys

txt = io.open('.staging-run.txt', encoding='utf-8').read()
m = re.search(r'^VOUCHER_JSON=(.*)$', txt, re.M)
voucher = json.loads(m.group(1))
auth = voucher['auth']

sig_bytes = bytes(voucher['signature'])
assert len(sig_bytes) == 65, f"sig len {len(sig_bytes)}"
sig = bytearray(sig_bytes)
if sig[64] < 27:
    sig[64] += 27
sig_hex = '0x' + bytes(sig).hex()

value_dec = str(int(auth['value'], 16))

body = {
    "x402Version": 1,
    "paymentPayload": {
        "x402Version": 1,
        "scheme": "exact",
        "network": "base-sepolia",
        "payload": {
            "signature": sig_hex,
            "authorization": {
                "from": auth['from'],
                "to": auth['to'],
                "value": value_dec,
                "validAfter": str(auth['valid_after']),
                "validBefore": str(auth['valid_before']),
                "nonce": auth['nonce'],
            },
        },
    },
    "paymentRequirements": {
        "scheme": "exact",
        "network": "base-sepolia",
        "maxAmountRequired": "5000",
        "resource": "https://code402-edge.akrivis.workers.dev/v1/tools/vat-mod97-check/call",
        "description": "code402 tool call",
        "mimeType": "application/json",
        "payTo": auth['to'],
        "maxTimeoutSeconds": 300,
        "asset": "0x036CbD53842c5426634e7929541eC2318f3dCF7e",
        "extra": {"name": "USDC", "version": "2"},
    },
}

with io.open('.staging-settle-body.json', 'w', encoding='utf-8', newline='') as f:
    json.dump(body, f, separators=(',', ':'))
print("value_dec:", value_dec, "v_last_byte:", sig[64], "sig_len_hex:", len(sig_hex))
