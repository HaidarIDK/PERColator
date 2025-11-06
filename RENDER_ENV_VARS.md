# Render Environment Variables Setup

## Frontend Environment Variables (Next.js)

Set these in your Render Frontend service:

```
NEXT_PUBLIC_API_URL=https://your-backend-url.onrender.com
NEXT_PUBLIC_WS_URL=wss://your-backend-url.onrender.com/ws
NEXT_PUBLIC_SOLANA_NETWORK=devnet
NEXT_PUBLIC_SOLANA_RPC_URL=https://api.devnet.solana.com
NEXT_PUBLIC_SLAB_PROGRAM_ID=YourSlabProgramId
NEXT_PUBLIC_SLAB_ACCOUNT=YourSlabAccount
NEXT_PUBLIC_INSTRUMENT_ID=YourInstrumentId
NEXT_PUBLIC_ROUTER_PROGRAM_ID=YourRouterProgramId
NEXT_PUBLIC_ROUTER_REGISTRY=YourRouterRegistry
NEXT_PUBLIC_AUTHORITY=YourAuthority
```

## Backend Environment Variables (Node.js/Express)

Set these in your Render Backend service:

```
PORT=10000
SOLANA_NETWORK=devnet
SOLANA_RPC_URL=https://api.devnet.solana.com
FRONTEND_URL=https://your-frontend-url.onrender.com
SLAB_PROGRAM_ID=YourSlabProgramId
SLAB_ACCOUNT=YourSlabAccount
ROUTER_PROGRAM_ID=YourRouterProgramId
ROUTER_ACCOUNT=YourRouterAccount
ROUTER_VAULT_ACCOUNT=YourRouterVaultAccount
PORTFOLIO_ACCOUNT=YourPortfolioAccount
ESCROW_ACCOUNT=YourEscrowAccount
```

## Important Notes

1. **Replace placeholders**: Replace `your-backend-url` and `your-frontend-url` with your actual Render URLs
2. **WebSocket URLs**: Use `wss://` (secure WebSocket) for production, not `ws://`
3. **CORS**: The backend already allows requests from Render URLs automatically
4. **After updating**: Redeploy both services after changing environment variables

## How to Update in Render

1. Go to your Render dashboard
2. Select your service (Frontend or Backend)
3. Go to "Environment" tab
4. Click "Add Environment Variable"
5. Add each variable listed above
6. Click "Save Changes"
7. Render will automatically redeploy


