const RPC_URL = process.env.NEXT_PUBLIC_RPC_URL || 'http://35.184.213.248:8545';
const REST_URL = process.env.NEXT_PUBLIC_REST_URL || 'http://35.184.213.248:8080';

let nextId = 1;

export async function rpc(method: string, params: unknown[] = []): Promise<unknown> {
  const res = await fetch(RPC_URL, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      jsonrpc: '2.0',
      id: nextId++,
      method,
      params,
    }),
  });
  const json = await res.json();
  if (json.error) throw new Error(json.error.message);
  return json.result;
}

export async function rest(path: string): Promise<unknown> {
  const res = await fetch(`${REST_URL}${path}`);
  return res.json();
}

export async function getBalance(address: string): Promise<number> {
  const result = (await rpc('dina_getBalance', [address])) as number;
  return result;
}

export async function getHealth(): Promise<{ height: number; status: string }> {
  return rest('/health') as Promise<{ height: number; status: string }>;
}

export async function getNetworkInfo(): Promise<unknown> {
  return rpc('dina_networkInfo');
}
