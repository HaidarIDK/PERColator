import { Router, Request, Response } from 'express';

const router = Router();

/**
 * POST /api/ai/chat
 * 
 * AI Chat endpoint that can be connected to OpenAI, Claude, or any other AI service
 * 
 * Request body:
 * {
 *   question: string,
 *   context: {
 *     wallet: { connected: boolean, address?: string },
 *     cliHistory: string[],
 *     availableCommands: string[],
 *     timestamp: string
 *   },
 *   chatHistory: Array<{ role: 'user' | 'assistant', text: string }>
 * }
 */
router.post('/chat', async (req: Request, res: Response) => {
  try {
    const { question, context, chatHistory } = req.body;

    if (!question || !context) {
      return res.status(400).json({
        success: false,
        error: 'Missing required fields: question, context',
      });
    }

    // Build system prompt with context
    const systemPrompt = buildSystemPrompt(context);

    // Build conversation history
    const messages = buildMessages(systemPrompt, chatHistory, question);

    // OPTION 1: Use OpenAI
    // const response = await callOpenAI(messages);

    // OPTION 2: Use Anthropic Claude
    // const response = await callClaude(messages);

    // OPTION 3: Use any other AI service
    // const response = await callCustomAI(messages);

    // For now, return a placeholder response
    const aiResponse = {
      success: true,
      response: `[AI Backend] I received your question: "${question}"\n\nContext:\n- Wallet: ${context.wallet.connected ? 'Connected' : 'Not connected'}\n- Commands available: ${context.availableCommands.length}\n\nTo enable real AI, uncomment the API calls in website/api/src/routes/ai.ts`,
      timestamp: new Date().toISOString(),
    };

    return res.json(aiResponse);

  } catch (error) {
    console.error('AI chat error:', error);
    return res.status(500).json({
      success: false,
      error: 'Internal server error',
    });
  }
});

/**
 * Build system prompt with context
 */
function buildSystemPrompt(context: any): string {
  return `You are an AI assistant for Percolator, a decentralized perpetual futures exchange on Solana.

PLATFORM KNOWLEDGE:
- Percolator is a decentralized perpetual futures DEX on Solana
- Uses on-chain orderbooks (called "slabs") for limit orders
- Uses AMM for instant liquidity
- Supports SOL/USDC, ETH/USDC, BTC/USDC trading pairs
- All trades are fully on-chain and transparent

KEY CONCEPTS:
- Slab: On-chain orderbook where LPs post orders and takers execute
- Tick Size: Minimum price increment (e.g., $0.01)
- Lot Size: Minimum quantity increment (e.g., 0.01 SOL)
- Oracle: Real-time price feeds from CoinGecko
- Portfolio: User's account with positions and balances

AVAILABLE CLI COMMANDS:
${context.availableCommands.map((cmd: string) => `- ${cmd}`).join('\n')}

USER CONTEXT:
- Wallet Connected: ${context.wallet.connected ? 'YES' : 'NO'}
${context.wallet.address ? `- Wallet Address: ${context.wallet.address}` : ''}
- Recent Activity: ${context.cliHistory.length > 0 ? 'Yes' : 'No recent commands'}
${context.cliHistory.length > 0 ? `- Last Commands:\n${context.cliHistory.slice(-5).map((h: string) => `  ${h}`).join('\n')}` : ''}

YOUR ROLE:
- Help users understand and use the CLI
- Explain trading concepts clearly
- Troubleshoot issues based on context
- Provide actionable, specific guidance
- Be concise but informative

STYLE:
- Use bullet points for clarity
- Include ✓/✗ symbols for status
- Provide examples when helpful
- Check user's wallet connection status before suggesting commands
- Reference their recent commands when relevant`;
}

/**
 * Build message array for AI
 */
function buildMessages(systemPrompt: string, chatHistory: any[], question: string): any[] {
  const messages = [
    { role: 'system', content: systemPrompt },
  ];

  // Add conversation history (last 5 messages for context)
  if (chatHistory && chatHistory.length > 0) {
    chatHistory.slice(-5).forEach(msg => {
      messages.push({
        role: msg.role === 'user' ? 'user' : 'assistant',
        content: msg.text,
      });
    });
  }

  // Add current question
  messages.push({
    role: 'user',
    content: question,
  });

  return messages;
}

/**
 * Call OpenAI API (uncomment and configure to use)
 */
/*
async function callOpenAI(messages: any[]): Promise<string> {
  const OPENAI_API_KEY = process.env.OPENAI_API_KEY;
  
  if (!OPENAI_API_KEY) {
    throw new Error('OPENAI_API_KEY not set in environment');
  }

  const response = await fetch('https://api.openai.com/v1/chat/completions', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${OPENAI_API_KEY}`,
    },
    body: JSON.stringify({
      model: 'gpt-4-turbo-preview', // or 'gpt-3.5-turbo' for faster/cheaper
      messages: messages,
      temperature: 0.7,
      max_tokens: 500,
    }),
  });

  const data = await response.json();
  
  if (!response.ok) {
    throw new Error(`OpenAI API error: ${data.error?.message || 'Unknown error'}`);
  }

  return data.choices[0].message.content;
}
*/

/**
 * Call Anthropic Claude API (uncomment and configure to use)
 */
/*
async function callClaude(messages: any[]): Promise<string> {
  const ANTHROPIC_API_KEY = process.env.ANTHROPIC_API_KEY;
  
  if (!ANTHROPIC_API_KEY) {
    throw new Error('ANTHROPIC_API_KEY not set in environment');
  }

  // Extract system message and conversation
  const systemMessage = messages.find(m => m.role === 'system')?.content || '';
  const conversationMessages = messages.filter(m => m.role !== 'system');

  const response = await fetch('https://api.anthropic.com/v1/messages', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'x-api-key': ANTHROPIC_API_KEY,
      'anthropic-version': '2023-06-01',
    },
    body: JSON.stringify({
      model: 'claude-3-sonnet-20240229', // or 'claude-3-opus-20240229' for better quality
      system: systemMessage,
      messages: conversationMessages,
      max_tokens: 500,
    }),
  });

  const data = await response.json();
  
  if (!response.ok) {
    throw new Error(`Claude API error: ${data.error?.message || 'Unknown error'}`);
  }

  return data.content[0].text;
}
*/

export default router;

