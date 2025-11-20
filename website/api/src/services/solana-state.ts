import { Connection, PublicKey, LAMPORTS_PER_SOL } from '@solana/web3.js';

export interface AccountBalance {
  address: string;
  lamports: number;
  sol: number;
}

export interface NetworkStatus {
  network: string;
  rpcUrl: string;
  blockHeight: number;
  healthy: boolean;
  slot: number;
}

export interface ProgramAccount {
  pubkey: string;
  lamports: number;
  owner: string;
  executable: boolean;
  dataSize: number;
}

/**
 * Solana State Service
 * Provides real-time queries to Solana blockchain state
 */
export class SolanaStateService {
  private connections: Map<string, Connection> = new Map();

  /**
   * Get or create a connection for a network
   */
  private getConnection(network: string, customRpcUrl?: string): Connection {
    const rpcUrl = customRpcUrl || this.getDefaultRpcUrl(network);
    const key = `${network}:${rpcUrl}`;

    if (!this.connections.has(key)) {
      this.connections.set(key, new Connection(rpcUrl, 'confirmed'));
    }

    return this.connections.get(key)!;
  }

  /**
   * Get default RPC URL for a network
   */
  private getDefaultRpcUrl(network: string): string {
    switch (network.toLowerCase()) {
      case 'localnet':
      case 'local':
        return 'http://127.0.0.1:8899';
      case 'devnet':
        return 'https://api.devnet.solana.com';
      case 'mainnet-beta':
      case 'mainnet':
        return 'https://api.mainnet-beta.solana.com';
      default:
        return 'http://127.0.0.1:8899';
    }
  }

  /**
   * Get balance for an address
   */
  async getBalance(
    address: string,
    network: string = 'localnet',
    rpcUrl?: string
  ): Promise<AccountBalance> {
    const connection = this.getConnection(network, rpcUrl);
    const pubkey = new PublicKey(address);

    const lamports = await connection.getBalance(pubkey);

    return {
      address,
      lamports,
      sol: lamports / LAMPORTS_PER_SOL,
    };
  }

  /**
   * Get network status and health
   */
  async getNetworkStatus(
    network: string = 'localnet',
    rpcUrl?: string
  ): Promise<NetworkStatus> {
    const connection = this.getConnection(network, rpcUrl);
    const url = rpcUrl || this.getDefaultRpcUrl(network);

    try {
      const [blockHeight, slot] = await Promise.all([
        connection.getBlockHeight(),
        connection.getSlot(),
      ]);

      return {
        network,
        rpcUrl: url,
        blockHeight,
        slot,
        healthy: true,
      };
    } catch (error) {
      return {
        network,
        rpcUrl: url,
        blockHeight: 0,
        slot: 0,
        healthy: false,
      };
    }
  }

  /**
   * Get program account information
   */
  async getProgramAccount(
    programId: string,
    network: string = 'localnet',
    rpcUrl?: string
  ): Promise<ProgramAccount | null> {
    const connection = this.getConnection(network, rpcUrl);
    const pubkey = new PublicKey(programId);

    try {
      const accountInfo = await connection.getAccountInfo(pubkey);

      if (!accountInfo) {
        return null;
      }

      return {
        pubkey: programId,
        lamports: accountInfo.lamports,
        owner: accountInfo.owner.toBase58(),
        executable: accountInfo.executable,
        dataSize: accountInfo.data.length,
      };
    } catch (error) {
      console.error('Error fetching program account:', error);
      return null;
    }
  }

  /**
   * Check if an account exists
   */
  async accountExists(
    address: string,
    network: string = 'localnet',
    rpcUrl?: string
  ): Promise<boolean> {
    const connection = this.getConnection(network, rpcUrl);
    const pubkey = new PublicKey(address);

    try {
      const accountInfo = await connection.getAccountInfo(pubkey);
      return accountInfo !== null;
    } catch (error) {
      return false;
    }
  }

  /**
   * Get multiple account balances at once
   */
  async getMultipleBalances(
    addresses: string[],
    network: string = 'localnet',
    rpcUrl?: string
  ): Promise<AccountBalance[]> {
    const connection = this.getConnection(network, rpcUrl);

    const pubkeys = addresses.map((addr) => new PublicKey(addr));
    const accountInfos = await connection.getMultipleAccountsInfo(pubkeys);

    return addresses.map((address, index) => {
      const lamports = accountInfos[index]?.lamports || 0;
      return {
        address,
        lamports,
        sol: lamports / LAMPORTS_PER_SOL,
      };
    });
  }

  /**
   * Get recent block hash (useful for transaction building)
   */
  async getRecentBlockhash(
    network: string = 'localnet',
    rpcUrl?: string
  ): Promise<string> {
    const connection = this.getConnection(network, rpcUrl);
    const { blockhash } = await connection.getLatestBlockhash();
    return blockhash;
  }

  /**
   * Airdrop SOL to an address (localnet/devnet only)
   */
  async airdrop(
    address: string,
    amount: number = 10,
    network: string = 'localnet',
    rpcUrl?: string
  ): Promise<string> {
    const connection = this.getConnection(network, rpcUrl);
    const pubkey = new PublicKey(address);

    // Only allow airdrops on localnet and devnet
    if (!['localnet', 'local', 'devnet'].includes(network.toLowerCase())) {
      throw new Error('Airdrops only available on localnet and devnet');
    }

    const signature = await connection.requestAirdrop(
      pubkey,
      amount * LAMPORTS_PER_SOL
    );

    // Wait for confirmation
    await connection.confirmTransaction(signature);

    return signature;
  }

  /**
   * Get transaction signature status
   */
  async getSignatureStatus(
    signature: string,
    network: string = 'localnet',
    rpcUrl?: string
  ): Promise<any> {
    const connection = this.getConnection(network, rpcUrl);
    return await connection.getSignatureStatus(signature);
  }

  /**
   * Subscribe to account changes (for WebSocket streaming)
   */
  subscribeToAccount(
    address: string,
    callback: (accountInfo: any) => void,
    network: string = 'localnet',
    rpcUrl?: string
  ): number {
    const connection = this.getConnection(network, rpcUrl);
    const pubkey = new PublicKey(address);

    return connection.onAccountChange(pubkey, callback, 'confirmed');
  }

  /**
   * Unsubscribe from account changes
   */
  async unsubscribeFromAccount(
    subscriptionId: number,
    network: string = 'localnet',
    rpcUrl?: string
  ): Promise<void> {
    const connection = this.getConnection(network, rpcUrl);
    await connection.removeAccountChangeListener(subscriptionId);
  }
}

// Singleton instance
export const solanaState = new SolanaStateService();
