import { spawn } from 'child_process';

/**
 * Auto-fund an account with SOL if needed
 */
export async function ensureAccountFunded(address: string, minBalance: number = 10): Promise<boolean> {
  try {
    // Check current balance
    const balanceResult = await execSolanaCommand(['balance', address, '--lamports']);
    const balance = parseInt(balanceResult.trim()) || 0;
    const balanceInSol = balance / 1_000_000_000;

    console.log(`Account ${address} has ${balanceInSol} SOL`);

    if (balanceInSol >= minBalance) {
      return true;
    }

    // Need to fund
    const needed = minBalance - balanceInSol;
    console.log(`Funding account ${address} with ${needed} SOL...`);

    // Try airdrop
    try {
      await execSolanaCommand(['airdrop', Math.ceil(needed).toString(), address]);
      console.log(`✓ Airdrop successful`);
      return true;
    } catch (airdropError) {
      console.error('Airdrop failed:', airdropError);
      
      // Try multiple smaller airdrops
      for (let i = 0; i < Math.ceil(needed / 2); i++) {
        try {
          await execSolanaCommand(['airdrop', '2', address]);
          await sleep(3000); // Wait between airdrops
        } catch (e) {
          console.error(`Airdrop attempt ${i + 1} failed`);
        }
      }
    }

    // Check balance again
    const newBalanceResult = await execSolanaCommand(['balance', address, '--lamports']);
    const newBalance = parseInt(newBalanceResult.trim()) || 0;
    const newBalanceInSol = newBalance / 1_000_000_000;

    return newBalanceInSol >= minBalance;
  } catch (error) {
    console.error('Error ensuring account funded:', error);
    return false;
  }
}

/**
 * Execute a Solana CLI command
 */
function execSolanaCommand(args: string[]): Promise<string> {
  return new Promise((resolve, reject) => {
    const child = spawn('solana', args);
    let stdout = '';
    let stderr = '';

    child.stdout.on('data', (data) => {
      stdout += data.toString();
    });

    child.stderr.on('data', (data) => {
      stderr += data.toString();
    });

    child.on('close', (code) => {
      if (code === 0) {
        resolve(stdout);
      } else {
        reject(new Error(stderr || stdout || `Command failed with code ${code}`));
      }
    });

    child.on('error', (error) => {
      reject(error);
    });
  });
}

/**
 * Sleep utility
 */
function sleep(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms));
}

/**
 * Get the default payer address
 */
export async function getDefaultPayerAddress(): Promise<string | null> {
  try {
    const result = await execSolanaCommand(['address']);
    return result.trim();
  } catch (error) {
    console.error('Error getting default payer:', error);
    return null;
  }
}



