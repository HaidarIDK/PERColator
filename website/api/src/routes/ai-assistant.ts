import { Router } from 'express';

const router = Router();

/**
 * AI Assistant - Convert natural language to CLI commands
 * POST /api/ai-assistant/suggest
 */
router.post('/suggest', async (req, res) => {
  try {
    const { question } = req.body;

    if (!question || typeof question !== 'string') {
      return res.status(400).json({
        error: 'Question is required',
      });
    }

    console.log('AI Assistant question:', question);

    // Simple pattern matching for now (can be replaced with actual AI later)
    const suggestion = getSuggestion(question.toLowerCase());

    return res.json({
      question,
      suggestion: suggestion.command,
      explanation: suggestion.explanation,
      example: suggestion.example,
    });
  } catch (error) {
    console.error('AI Assistant error:', error);
    return res.status(500).json({
      error: error instanceof Error ? error.message : 'Failed to process question',
    });
  }
});

/**
 * Get command suggestion based on natural language
 */
function getSuggestion(question: string): { command: string; explanation: string; example: string } {
  // Initialize/Setup
  if (question.includes('initialize') || question.includes('init') || question.includes('setup') || question.includes('start')) {
    return {
      command: 'init --name MyExchange',
      explanation: 'Initialize a new exchange registry. This creates the central registry that all other components connect to.',
      example: 'init --name MyExchange',
    };
  }

  // Create slab/orderbook
  if (question.includes('create') && (question.includes('slab') || question.includes('orderbook') || question.includes('market'))) {
    return {
      command: 'matcher create --tick-size 100 --lot-size 1000 <EXCHANGE_ADDRESS> SOL-PERP',
      explanation: 'Create a new slab (orderbook) for a market. You need the exchange address from the init command.',
      example: 'matcher create --tick-size 100 --lot-size 1000 5EE1k9YhXTtQmYd4QXK7MRdkXtCkF2yDhLSxxZuVxeRr SOL-PERP',
    };
  }

  // List markets/slabs
  if (question.includes('list') || question.includes('show all') || question.includes('see all')) {
    if (question.includes('market') || question.includes('slab')) {
      return {
        command: 'matcher list <EXCHANGE_ADDRESS>',
        explanation: 'List all matchers/slabs for an exchange',
        example: 'matcher list 5EE1k9YhXTtQmYd4QXK7MRdkXtCkF2yDhLSxxZuVxeRr',
      };
    }
  }

  // Place order
  if (question.includes('place') && question.includes('order')) {
    return {
      command: 'matcher place-order --side <buy|sell> --price <PRICE> --size <SIZE> <SLAB_ADDRESS>',
      explanation: 'Place a limit order on the orderbook',
      example: 'matcher place-order --side buy --price 100000 --size 1000 <SLAB_ADDRESS>',
    };
  }

  // Get orderbook
  if (question.includes('orderbook') || question.includes('order book') || question.includes('book')) {
    return {
      command: 'matcher get-orderbook <SLAB_ADDRESS>',
      explanation: 'View the current state of the orderbook with all bids and asks',
      example: 'matcher get-orderbook 3tqHPaFNrsr51MUPPLXATJNAzApsFA2n2X9UpbcSrs4E',
    };
  }

  // Deploy
  if (question.includes('deploy')) {
    return {
      command: 'deploy',
      explanation: 'Deploy all Percolator programs to the network',
      example: 'deploy',
    };
  }

  // Test
  if (question.includes('test')) {
    return {
      command: 'test --help',
      explanation: 'Run tests. Use --help to see all test options like --basic, --crisis, --kitchen-sink',
      example: 'test --crisis',
    };
  }

  // Status
  if (question.includes('status') || question.includes('info') || question.includes('details')) {
    return {
      command: 'status <EXCHANGE_ADDRESS>',
      explanation: 'Show protocol status and statistics for an exchange',
      example: 'status 5EE1k9YhXTtQmYd4QXK7MRdkXtCkF2yDhLSxxZuVxeRr',
    };
  }

  // Trading
  if (question.includes('trade') || question.includes('buy') || question.includes('sell')) {
    return {
      command: 'trade --help',
      explanation: 'View trading operations. You can place orders, cancel orders, and more.',
      example: 'trade --help',
    };
  }

  // Liquidity
  if (question.includes('liquidity') || question.includes('lp') || question.includes('provide')) {
    return {
      command: 'liquidity --help',
      explanation: 'Liquidity provider operations. Add/remove liquidity from pools.',
      example: 'liquidity --help',
    };
  }

  // AMM
  if (question.includes('amm') || question.includes('swap')) {
    return {
      command: 'amm --help',
      explanation: 'Automated Market Maker operations. Create pools, add liquidity, swap tokens.',
      example: 'amm --help',
    };
  }

  // Wallet
  if (question.includes('wallet') || question.includes('balance') || question.includes('address')) {
    return {
      command: 'Use: solana address (to see wallet) or solana balance (to check balance)',
      explanation: 'Wallet operations use the standard Solana CLI commands',
      example: 'solana balance',
    };
  }

  // Help
  if (question.includes('help') || question.includes('how') || question.includes('what can')) {
    return {
      command: '--help',
      explanation: 'Show all available commands and their usage',
      example: '--help',
    };
  }

  // Default
  return {
    command: '--help',
    explanation: 'I\'m not sure about that. Try using --help to see all available commands, or ask me about: initialize, create slab, place order, deploy, test, etc.',
    example: '--help',
  };
}

export { router as aiAssistantRouter };





