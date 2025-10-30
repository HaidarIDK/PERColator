# Program Keypairs

This directory contains the keypairs for all on-chain programs.

## Programs

- **program-amm.json** - AMM (Automated Market Maker) program keypair
- **program-router.json** - Router program keypair (v1 with slab support)
- **program-slab.json** - Slab program keypair (v1)
- **program-oracle.json** - Oracle program keypair

## Usage

To get the program ID (public key) from a keypair:
```bash
solana-keygen pubkey scripts/keypairs/program-amm.json
```

## Security

⚠️ **Keep these keypairs secure!** They contain private keys for your deployed programs.
Do NOT commit these to git (they should be in .gitignore).

