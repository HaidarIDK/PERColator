import fs from 'fs';
import path from 'path';
import { Keypair, PublicKey } from '@solana/web3.js';

export interface ProgramIDs {
  router: string;
  slab: string;
  amm: string;
  oracle: string;
}

export interface ProgramInfo {
  name: string;
  programId: string;
  keypairPath: string;
  exists: boolean;
}

/**
 * Program Discovery Service
 * Reads program IDs from deployed keypair files
 */
export class ProgramDiscoveryService {
  private readonly projectRoot: string;
  private programIds: ProgramIDs | null = null;
  private lastRefresh: number = 0;
  private readonly cacheTimeout = 60000; // 1 minute cache

  constructor(projectRoot?: string) {
    // Auto-detect project root (go up from api directory)
    this.projectRoot = projectRoot || path.join(__dirname, '../../../..');
  }

  /**
   * Get program IDs with caching
   */
  async getProgramIDs(): Promise<ProgramIDs> {
    const now = Date.now();

    // Return cached if still valid
    if (this.programIds && (now - this.lastRefresh) < this.cacheTimeout) {
      return this.programIds;
    }

    // Refresh program IDs
    this.programIds = await this.loadProgramIDs();
    this.lastRefresh = now;

    return this.programIds;
  }

  /**
   * Load program IDs from keypair files
   */
  private async loadProgramIDs(): Promise<ProgramIDs> {
    const deployDir = path.join(this.projectRoot, 'target', 'deploy');

    const programs = {
      router: 'percolator_router-keypair.json',
      slab: 'percolator_slab-keypair.json',
      amm: 'percolator_amm-keypair.json',
      oracle: 'percolator_oracle-keypair.json',
    };

    const ids: any = {};

    for (const [name, filename] of Object.entries(programs)) {
      const keypairPath = path.join(deployDir, filename);
      try {
        const programId = await this.loadProgramIdFromKeypair(keypairPath);
        ids[name] = programId;
      } catch (error) {
        console.warn(`Warning: Could not load ${name} program ID from ${keypairPath}:`, error);
        ids[name] = null;
      }
    }

    return ids as ProgramIDs;
  }

  /**
   * Load a program ID by reading its keypair file
   */
  private async loadProgramIdFromKeypair(keypairPath: string): Promise<string> {
    if (!fs.existsSync(keypairPath)) {
      throw new Error(`Keypair file not found: ${keypairPath}`);
    }

    const keypairData = fs.readFileSync(keypairPath, 'utf-8');
    const secretKey = Uint8Array.from(JSON.parse(keypairData));
    const keypair = Keypair.fromSecretKey(secretKey);

    return keypair.publicKey.toBase58();
  }

  /**
   * Get detailed information about all programs
   */
  async getProgramInfo(): Promise<ProgramInfo[]> {
    const deployDir = path.join(this.projectRoot, 'target', 'deploy');

    const programs = [
      { name: 'router', filename: 'percolator_router-keypair.json' },
      { name: 'slab', filename: 'percolator_slab-keypair.json' },
      { name: 'amm', filename: 'percolator_amm-keypair.json' },
      { name: 'oracle', filename: 'percolator_oracle-keypair.json' },
    ];

    const info: ProgramInfo[] = [];

    for (const program of programs) {
      const keypairPath = path.join(deployDir, program.filename);
      const exists = fs.existsSync(keypairPath);

      let programId = 'Not deployed';
      if (exists) {
        try {
          programId = await this.loadProgramIdFromKeypair(keypairPath);
        } catch (error) {
          programId = 'Error loading';
        }
      }

      info.push({
        name: program.name,
        programId,
        keypairPath,
        exists,
      });
    }

    return info;
  }

  /**
   * Force refresh of program IDs (useful after deployment)
   */
  async refresh(): Promise<ProgramIDs> {
    this.lastRefresh = 0;
    return this.getProgramIDs();
  }

  /**
   * Check if all programs are deployed
   */
  async allProgramsDeployed(): Promise<boolean> {
    const ids = await this.getProgramIDs();
    return !!(ids.router && ids.slab && ids.amm && ids.oracle);
  }

  /**
   * Get the project root directory
   */
  getProjectRoot(): string {
    return this.projectRoot;
  }
}

// Singleton instance
export const programDiscovery = new ProgramDiscoveryService();
