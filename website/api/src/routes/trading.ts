import { Router } from 'express';
import { PublicKey, Transaction, SystemProgram, LAMPORTS_PER_SOL } from '@solana/web3.js';
import {
  buildReserveInstruction,
  buildCommitInstruction,
  buildMultiReserveInstruction,
  buildMultiCommitInstruction,
  generateSecret,
  generateCommitmentHash,
  serializeTransaction,
  simulateTransaction,
  getRecentBlockhash,
  SLAB_PROGRAM_ID,
  ROUTER_PROGRAM_ID,
  SLAB_ACCOUNT,
} from '../services/transactions';
import { logTransaction } from './monitor';

export const tradingRouter = Router();

// Store active reservations (in production, use Redis or database)
const activeReservations = new Map<string, {
  holdId: number;
  secret: Buffer;
  vwapPrice: number;
  worstPrice: number;
  maxCharge: number;
  filledQty: number;
  expiryMs: number;
}>();

// Store completed trades for orderbook display
export const completedTrades: Array<{
  timestamp: number;
  user: string;
  side: 'buy' | 'sell';
  price: number;
  quantity: number;
  signature?: string;
}> = [];

// Store active orders (reservations that haven't been committed yet)
export const activeOrders: Array<{
  holdId: number;
  user: string;
  side: 'buy' | 'sell';
  price: number;
  quantity: number;
  timestamp: number;
  expiryMs: number;
}> = [];

/**
 * POST /api/trade/order
 * Simplified order placement (hides reserve-commit complexity)
 * User just wants to buy or sell - we handle the two-phase execution
 */
tradingRouter.post('/order', async (req, res) => {
  try {
    const {
      user,
      side, // 'buy' or 'sell'
      price,
      quantity,
      orderType = 'limit',
      instrument = 0,
    } = req.body;

    if (!user || !quantity) {
      return res.status(400).json({ error: 'Missing required fields: user, quantity' });
    }

    if (orderType === 'limit' && !price) {
      return res.status(400).json({ error: 'Price required for limit orders' });
    }

    // For now, just log the order (mock mode)
    // In production, this would call reserve-commit automatically
    const holdId = Math.floor(Math.random() * 1000000);
    
    console.log(`📝 Order placed: ${side} ${quantity} @ ${price || 'market'}`);
    
    // Log to monitor
    const { logTransaction } = await import('./monitor');
    logTransaction({
      id: Date.now(),
      type: 'trade',
      timestamp: Date.now(),
      user: user.substring(0, 6) + '...' + user.substring(user.length - 4),
      action: `${side === 'buy' ? 'Bought' : 'Sold'} ${quantity} @ ${price || 'market'}`,
      amount: price ? parseFloat(quantity) * parseFloat(price) : 0,
      data: {
        side,
        qty: quantity,
        price,
        instrument,
      },
      signature: `Order${holdId}`,
    });

    res.json({
      success: true,
      orderId: holdId,
      side,
      quantity,
      price,
      status: 'placed',
      message: `${side === 'buy' ? 'Buy' : 'Sell'} order placed successfully!`,
      note: 'Order is now in the book (mock mode - will be real after deployment)'
    });

  } catch (error: any) {
    console.error('Order error:', error);
    res.status(500).json({ error: error.message });
  }
});

/**
 * POST /api/trade/reserve
 * Reserve liquidity (two-phase execution, step 1)
 * Returns an unsigned transaction for the user to sign
 */
tradingRouter.post('/reserve', async (req, res) => {
  try {
    const {
      user,
      slice,
      orderType,
      price,
      quantity,
      capLimit,
      multiAsset,
      instrument = 0,
      side = 'buy',
    } = req.body;

    if (!user || !price || !quantity) {
      return res.status(400).json({ error: 'Missing required fields: user, price, quantity' });
    }

    // Parse user public key
    let userPubkey: PublicKey;
    try {
      userPubkey = new PublicKey(user);
    } catch (error) {
      return res.status(400).json({ error: 'Invalid user public key' });
    }

    // Get slab account from environment or use initialized account
    const slabAccount = SLAB_ACCOUNT;

    // Generate commitment secret for commit-reveal pattern
    const secret = generateSecret();
    const commitmentHash = generateCommitmentHash(secret);

    // Build reserve instruction
    // POC NOTE: Using accountIdx 0 - in production this would:
    // 1. Check if user has an account in the slab's account pool
    // 2. Auto-create one if not (requires program modification)
    // 3. Or use a separate "Register Account" instruction first
    const reserveIx = buildReserveInstruction({
      slabAccount,
      userAccount: userPubkey,
      accountIdx: 0, // POC: Using index 0 (needs account pool modification in program)
      instrumentIdx: instrument,
      side: side as 'buy' | 'sell',
      qty: quantity,
      limitPrice: price,
      ttlMs: 60000, // 60 seconds TTL
      commitmentHash,
      routeId: Date.now(), // Use timestamp as route ID (will be made unique in buildReserveInstruction)
    });

    // Create transaction
    const transaction = new Transaction();
    transaction.add(reserveIx);

    // IMPORTANT: Get FRESH blockhash for EACH transaction to ensure uniqueness
    // This prevents "already processed" errors when doing multiple trades
    const blockhash = await getRecentBlockhash();
    transaction.recentBlockhash = blockhash;
    transaction.feePayer = userPubkey;

    // Simulate transaction before sending to wallet (prevents Phantom warnings)
    const simulation = await simulateTransaction(transaction);
    if (!simulation.success) {
      return res.status(400).json({ 
        error: `Transaction would fail: ${simulation.error}`,
        simulationError: simulation.error
      });
    }

    // Serialize transaction for frontend
    const serializedTx = serializeTransaction(transaction);

    // Generate hold ID (in production, this would come from blockchain response)
    const holdId = Math.floor(Math.random() * 1000000);

    // Calculate estimated prices (mock for now, would come from slab state)
    const vwapPrice = price;
    const worstPrice = price * (side === 'buy' ? 1.001 : 0.999);
    const maxCharge = quantity * worstPrice;
    const expiryMs = Date.now() + 60000;

    // Store reservation details
    const reservationKey = `${user}-${holdId}`;
    activeReservations.set(reservationKey, {
      holdId,
      secret,
      vwapPrice,
      worstPrice,
      maxCharge,
      filledQty: quantity,
      expiryMs,
    });

    // Add to active orders for orderbook display
    activeOrders.push({
      holdId,
      user,
      side: side as 'buy' | 'sell',
      price,
      quantity,
      timestamp: Date.now(),
      expiryMs,
    });

    // Log to monitor
    logTransaction({
      id: Date.now(),
      type: 'reserve',
      timestamp: Date.now(),
      user: user.substring(0, 6) + '...' + user.substring(user.length - 4),
      data: {
        instrument,
        side,
        qty: quantity,
        price,
        hold_id: holdId,
      },
      signature: `Reserve${holdId}`,
    });

    // Return transaction and reservation details
    res.json({
      success: true,
      transaction: serializedTx,
      needsSigning: true,
      holdId,
      vwapPrice,
      worstPrice,
      maxCharge,
      expiryMs,
      reservedQty: quantity,
      message: 'Sign and submit this transaction to reserve liquidity',
    });
  } catch (error: any) {
    console.error('Reserve error:', error);
    res.status(500).json({ error: error.message });
  }
});

/**
 * POST /api/trade/commit
 * Commit a reservation (two-phase execution, step 2)
 * Returns an unsigned transaction for the user to sign
 */
tradingRouter.post('/commit', async (req, res) => {
  try {
    const { user, holdId } = req.body;

    if (!user || !holdId) {
      return res.status(400).json({ error: 'Missing required fields: user, holdId' });
    }

    // Parse user public key
    let userPubkey: PublicKey;
    try {
      userPubkey = new PublicKey(user);
    } catch (error) {
      return res.status(400).json({ error: 'Invalid user public key' });
    }

    // Get reservation details
    const reservationKey = `${user}-${holdId}`;
    const reservation = activeReservations.get(reservationKey);

    if (!reservation) {
      return res.status(404).json({ error: 'Reservation not found or expired' });
    }

    // Check if reservation has expired
    if (Date.now() > reservation.expiryMs) {
      activeReservations.delete(reservationKey);
      return res.status(400).json({ error: 'Reservation has expired' });
    }

    // Get slab account
    const slabAccount = SLAB_ACCOUNT;

    // Build commit instruction with revealed secret
    const commitIx = buildCommitInstruction({
      slabAccount,
      userAccount: userPubkey,
      holdId: reservation.holdId,
      commitmentReveal: reservation.secret,
    });

    // Create transaction
    const transaction = new Transaction();
    transaction.add(commitIx);

    // IMPORTANT: Get FRESH blockhash for EACH transaction to ensure uniqueness
    const blockhash = await getRecentBlockhash();
    transaction.recentBlockhash = blockhash;
    transaction.feePayer = userPubkey;

    // Simulate transaction before sending to wallet (prevents Phantom warnings)
    const simulation = await simulateTransaction(transaction);
    if (!simulation.success) {
      return res.status(400).json({ 
        error: `Transaction would fail: ${simulation.error}`,
        simulationError: simulation.error
      });
    }

    // Serialize transaction for frontend
    const serializedTx = serializeTransaction(transaction);

    // Log to monitor
    logTransaction({
      id: Date.now(),
      type: 'commit',
      timestamp: Date.now(),
      user: user.substring(0, 6) + '...' + user.substring(user.length - 4),
      data: {
        hold_id: reservation.holdId,
        filled_qty: reservation.filledQty,
        vwap_price: reservation.vwapPrice,
      },
      amount: reservation.filledQty * reservation.vwapPrice,
      signature: `Commit${reservation.holdId}`,
    });

    // Also log as a trade
    logTransaction({
      id: Date.now() + 1,
      type: 'trade',
      timestamp: Date.now(),
      user: user.substring(0, 6) + '...' + user.substring(user.length - 4),
      action: `Traded ${reservation.filledQty} @ ${reservation.vwapPrice}`,
      amount: reservation.filledQty * reservation.vwapPrice,
      signature: `Trade${reservation.holdId}`,
    });

    // Find and remove from active orders
    const orderIndex = activeOrders.findIndex(o => o.holdId === reservation.holdId);
    let orderData = null;
    if (orderIndex > -1) {
      orderData = activeOrders[orderIndex];
      activeOrders.splice(orderIndex, 1);
    }

    // Clean up reservation (in production, keep for a while for replay protection)
    activeReservations.delete(reservationKey);

    // Return transaction with order data for frontend to track
    res.json({
      success: true,
      transaction: serializedTx,
      needsSigning: true,
      holdId: reservation.holdId,
      vwapPrice: reservation.vwapPrice,
      filledQty: reservation.filledQty,
      totalCost: reservation.filledQty * reservation.vwapPrice,
      message: 'Sign and submit this transaction to commit the trade',
      orderData, // Include order data so frontend can add to completed trades
    });
  } catch (error: any) {
    console.error('Commit error:', error);
    res.status(500).json({ error: error.message });
  }
});

/**
 * POST /api/trade/multi-reserve
 * Reserve liquidity for multiple assets/slices
 */
tradingRouter.post('/multi-reserve', async (req, res) => {
  try {
    const {
      user,
      instruments,
      sides,
      quantities,
      limitPrices,
      ttlMs = 60000,
    } = req.body;

    if (!user || !instruments || !sides || !quantities || !limitPrices) {
      return res.status(400).json({
        error: 'Missing required fields: user, instruments, sides, quantities, limitPrices',
      });
    }

    // Parse user public key
    let userPubkey: PublicKey;
    try {
      userPubkey = new PublicKey(user);
    } catch (error) {
      return res.status(400).json({ error: 'Invalid user public key' });
    }

    // Get accounts (from environment or defaults)
    const routerAccount = new PublicKey(
      process.env.ROUTER_ACCOUNT || '11111111111111111111111111111111'
    );
    const escrowAccount = new PublicKey(
      process.env.ESCROW_ACCOUNT || '11111111111111111111111111111111'
    );
    const portfolioAccount = new PublicKey(
      process.env.PORTFOLIO_ACCOUNT || '11111111111111111111111111111111'
    );

    // TODO: Get actual cap and slab accounts
    const capAccounts: PublicKey[] = [];
    const slabAccounts = instruments.map(() => SLAB_ACCOUNT);

    // Generate commitment
    const secret = generateSecret();
    const commitmentHash = generateCommitmentHash(secret);

    // Build multi-reserve instruction
    const multiReserveIx = buildMultiReserveInstruction({
      routerAccount,
      escrowAccount,
      portfolioAccount,
      capAccounts,
      slabAccounts,
      userAuthority: userPubkey,
      instruments,
      sides,
      quantities,
      limitPrices,
      ttlMs,
      commitmentHash,
    });

    // Create transaction
    const transaction = new Transaction();
    transaction.add(multiReserveIx);

    // Get recent blockhash
    const blockhash = await getRecentBlockhash();
    transaction.recentBlockhash = blockhash;
    transaction.feePayer = userPubkey;

    // Serialize transaction
    const serializedTx = serializeTransaction(transaction);

    // Generate hold IDs for each order
    const holdIds = instruments.map(() => Math.floor(Math.random() * 1000000));

    // Store reservation
    const reservationKey = `${user}-multi-${Date.now()}`;
    activeReservations.set(reservationKey, {
      holdId: holdIds[0], // Store first hold ID as reference
      secret,
      vwapPrice: limitPrices[0],
      worstPrice: limitPrices[0],
      maxCharge: quantities.reduce((sum: number, qty: number, i: number) => sum + qty * limitPrices[i], 0),
      filledQty: quantities.reduce((sum: number, qty: number) => sum + qty, 0),
      expiryMs: Date.now() + ttlMs,
    });

    res.json({
      success: true,
      transaction: serializedTx,
      needsSigning: true,
      holdIds,
      reservationKey,
      expiryMs: Date.now() + ttlMs,
      message: 'Sign and submit this transaction to reserve liquidity across multiple assets',
    });
  } catch (error: any) {
    console.error('Multi-reserve error:', error);
    res.status(500).json({ error: error.message });
  }
});

/**
 * POST /api/trade/order
 * Place a new order (single-phase, for limit orders)
 */
tradingRouter.post('/order', async (req, res) => {
  try {
    const { user, instrument, side, price, qty } = req.body;

    if (!user || instrument === undefined || !side || !price || !qty) {
      return res.status(400).json({ error: 'Missing required fields' });
    }

    // For single-phase orders, we still use reserve mechanism
    // but with auto-commit (could be implemented as batch_open instruction)

    res.json({
      success: true,
      message: 'Use /reserve and /commit endpoints for two-phase execution',
      orderId: Math.floor(Math.random() * 1000000),
    });
  } catch (error: any) {
    res.status(500).json({ error: error.message });
  }
});

/**
 * POST /api/trade/cancel
 * Cancel a reservation before it expires
 */
tradingRouter.post('/cancel', async (req, res) => {
  try {
    const { user, holdId } = req.body;

    if (!user || !holdId) {
      return res.status(400).json({ error: 'Missing required fields' });
    }

    const reservationKey = `${user}-${holdId}`;
    const existed = activeReservations.delete(reservationKey);

    res.json({
      success: existed,
      message: existed ? 'Reservation cancelled' : 'Reservation not found',
    });
  } catch (error: any) {
    res.status(500).json({ error: error.message });
  }
});

/**
 * GET /api/trade/reservations/:user
 * Get active reservations for a user
 */
tradingRouter.get('/reservations/:user', async (req, res) => {
  try {
    const { user } = req.params;
    const userReservations: any[] = [];

    activeReservations.forEach((reservation, key) => {
      if (key.startsWith(user)) {
        userReservations.push({
          holdId: reservation.holdId,
          vwapPrice: reservation.vwapPrice,
          worstPrice: reservation.worstPrice,
          maxCharge: reservation.maxCharge,
          filledQty: reservation.filledQty,
          expiryMs: reservation.expiryMs,
          timeRemaining: Math.max(0, reservation.expiryMs - Date.now()),
        });
      }
    });

    res.json({
      success: true,
      reservations: userReservations,
    });
  } catch (error: any) {
    res.status(500).json({ error: error.message });
  }
});

/**
 * POST /api/trade/record-fill
 * Record a successful trade (called by frontend after commit confirmation)
 */
tradingRouter.post('/record-fill', async (req, res) => {
  try {
    const { user, side, price, quantity, signature } = req.body;

    if (!user || !side || !price || !quantity) {
      return res.status(400).json({ error: 'Missing required fields' });
    }

    // Add to completed trades
    completedTrades.push({
      timestamp: Date.now(),
      user,
      side,
      price,
      quantity,
      signature,
    });

    // Keep only last 100 trades
    if (completedTrades.length > 100) {
      completedTrades.shift();
    }

    res.json({ success: true, message: 'Trade recorded' });
  } catch (error: any) {
    res.status(500).json({ error: error.message });
  }
});

/**
 * POST /api/trade/test-transfer
 * Test endpoint - builds a simple SOL transfer transaction
 * This will ALWAYS succeed (no program deployment needed)
 */
tradingRouter.post('/test-transfer', async (req, res) => {
  try {
    const { user } = req.body;
    
    if (!user) {
      return res.status(400).json({ error: 'Missing user wallet address' });
    }

    const userPubkey = new PublicKey(user);
    
    // Create a simple transfer instruction (0.001 SOL to yourself)
    const transferInstruction = SystemProgram.transfer({
      fromPubkey: userPubkey,
      toPubkey: userPubkey, // Send to yourself
      lamports: 0.001 * LAMPORTS_PER_SOL, // 0.001 SOL
    });

    // Build transaction
    const transaction = new Transaction().add(transferInstruction);
    transaction.feePayer = userPubkey;
    transaction.recentBlockhash = await getRecentBlockhash();

    // Serialize and return
    const serializedTx = serializeTransaction(transaction);

    res.json({
      success: true,
      needsSigning: true,
      transaction: serializedTx,
      message: 'Simple test transaction - transfers 0.001 SOL to yourself',
      testMode: true,
    });
  } catch (error: any) {
    console.error('Error in /api/trade/test-transfer:', error);
    res.status(500).json({ error: error.message });
  }
});
