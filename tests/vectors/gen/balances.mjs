import { createPublicClient, http, formatEther, getAddress } from 'viem';
import { baseSepolia } from 'viem/chains';
import { privateKeyToAccount } from 'viem/accounts';
import { readFileSync } from 'node:fs';
const pub = createPublicClient({ chain: baseSepolia, transport: http('https://sepolia.base.org') });
const erc20 = [{ name:'balanceOf', type:'function', stateMutability:'view', inputs:[{name:'a',type:'address'}], outputs:[{type:'uint256'}] }];
for (const f of process.argv.slice(2)) {
  const k = '0x' + readFileSync(f, 'utf8').trim();
  const a = privateKeyToAccount(k).address;
  const eth = await pub.getBalance({ address: a });
  const usdc = await pub.readContract({ address: '0x036CbD53842c5426634e7929541eC2318f3dCF7e', abi: erc20, functionName: 'balanceOf', args: [a] });
  console.log(a, 'ETH=' + formatEther(eth), 'USDC=' + Number(usdc)/1e6);
}
