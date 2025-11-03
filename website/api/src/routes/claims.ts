import { Router } from 'express';

export const claimsRouter = Router();

/**
 * GET /api/claims/total-claimed
 * Get total claimed amount (for rewards/incentives)
 */
claimsRouter.get('/total-claimed', async (req, res) => {
  try {
    res.json({
      total_claimed: 1234567.89,
      total_claimable: 987654.32,
      num_claimants: 1523,
      last_claim_ts: Date.now() - 3600000,
      next_distribution_ts: Date.now() + 86400000,
    });
  } catch (error: any) {
    res.status(500).json({ error: error.message });
  }
});

/**
 * GET /api/claims/user/:user
 * Get user's claimable rewards
 */
claimsRouter.get('/user/:user', async (req, res) => {
  try {
    const { user } = req.params;
    
    res.json({
      user,
      claimable: 123.45,
      claimed: 567.89,
      pending: 45.67,
      rewards: [
        {
          type: 'trading_volume',
          amount: 50.00,
          earned_at: Date.now() - 7200000,
        },
        {
          type: 'liquidity_provision',
          amount: 73.45,
          earned_at: Date.now() - 3600000,
        }
      ]
    });
  } catch (error: any) {
    res.status(500).json({ error: error.message });
  }
});

/**
 * POST /api/claims/claim
 * Claim pending rewards
 */
claimsRouter.post('/claim', async (req, res) => {
  try {
    const { user, amount } = req.body;
    
    if (!user) {
      return res.status(400).json({ error: 'user required' });
    }

    res.json({
      success: true,
      user,
      amount: amount || 123.45,
      claimed_at: Date.now(),
      signature: 'MockClaimSignature' + Math.random().toString(36).substring(7),
    });
  } catch (error: any) {
    res.status(500).json({ error: error.message });
  }
});

