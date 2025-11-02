# AI Assistant Integration Guide

## Overview

The CLI page now includes an **AI Assistant** that can help users interact with the Percolator platform. The AI is context-aware and can answer questions based on:

- **Wallet Connection Status** - Whether user's wallet is connected
- **Wallet Address** - User's public key (if connected)
- **CLI Command History** - Last 10 commands/outputs
- **Available Commands** - All CLI commands the user can run
- **Conversation History** - Previous messages for context

---

## Current Implementation

### 🟢 What's Working Now

The AI uses a **smart rule-based system** that:
- Detects keywords in user questions
- Provides context-aware responses
- Shows wallet connection status
- References recent command history
- Explains concepts clearly

**No external API calls** - Everything runs locally!

---

## How Context is Gathered

### Frontend (website/frontend/src/app/cli/page.tsx)

```typescript
const context = {
  wallet: {
    connected: !!publicKey,
    address: publicKey?.toBase58(),
  },
  cliHistory: output.slice(-10).map(o => o.text), // Last 10 outputs
  availableCommands: Object.keys(availableCommands),
  timestamp: new Date().toISOString(),
};
```

### What the AI Sees

When a user asks: **"How do I check my portfolio?"**

The AI receives:
```json
{
  "question": "How do I check my portfolio?",
  "context": {
    "wallet": {
      "connected": true,
      "address": "BWFTKUYSPo3vKPAY9wSbAnH15wH4JyapTE4iA7Rs4MBV"
    },
    "cliHistory": [
      "$ help",
      "Available commands: portfolio, balance, markets...",
      "$ balance",
      "Balance: 2.45 SOL"
    ],
    "availableCommands": ["help", "portfolio", "balance", "markets", "orders", "history", "deposit", "withdraw", "trade", "lp", "clear"],
    "timestamp": "2025-11-02T10:30:00.000Z"
  },
  "chatHistory": [
    { "role": "assistant", "text": "Hi! I'm your Percolator AI Assistant..." },
    { "role": "user", "text": "What is a slab?" },
    { "role": "assistant", "text": "A slab is an on-chain orderbook..." }
  ]
}
```

The AI responds with:
```
✓ Your wallet is connected (BWFTKUYS...)

To check your portfolio, use the "portfolio" command.

This command will show:
• Your wallet address
• SOL balance
• Open positions
• Total equity

Example: Just type "portfolio" in the terminal above.
```

---

## Upgrading to Real AI

### Option 1: OpenAI (GPT-4 / GPT-3.5)

#### 1. Get API Key
- Go to https://platform.openai.com/api-keys
- Create a new API key
- Copy it

#### 2. Set Environment Variable

**In `website/api/.env`:**
```bash
OPENAI_API_KEY=sk-proj-xxxxxxxxxxxxxxxxxxxxx
```

#### 3. Enable in Frontend

**In `website/frontend/src/app/cli/page.tsx`:**
```typescript
// Uncomment these lines (around line 229-239):
const response = await fetch('/api/ai/chat', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    question: userQuestion,
    context: context,
    chatHistory: aiChat.slice(-5),
  }),
});
const data = await response.json();
const aiResponse = data.response;
```

Comment out:
```typescript
// const aiResponse = await generateAiResponse(userQuestion, context);
```

#### 4. Enable in Backend

**In `website/api/src/routes/ai.ts`:**

Uncomment the `callOpenAI` function (around line 108):
```typescript
const response = await callOpenAI(messages);
```

#### 5. Restart Backend
```bash
cd website/api
npm run dev
```

**Cost:** ~$0.002 per message (GPT-3.5) or ~$0.03 per message (GPT-4)

---

### Option 2: Anthropic Claude

#### 1. Get API Key
- Go to https://console.anthropic.com/
- Create API key
- Copy it

#### 2. Set Environment Variable

**In `website/api/.env`:**
```bash
ANTHROPIC_API_KEY=sk-ant-xxxxxxxxxxxxxxxxxxxxx
```

#### 3. Enable in Backend

**In `website/api/src/routes/ai.ts`:**

Uncomment the `callClaude` function (around line 149):
```typescript
const response = await callClaude(messages);
```

#### 4. Follow steps 3-5 from OpenAI section

**Cost:** ~$0.015 per message (Claude 3 Sonnet) or ~$0.075 per message (Claude 3 Opus)

---

### Option 3: Custom AI / Self-Hosted

You can use any AI service by implementing a custom function:

```typescript
async function callCustomAI(messages: any[]): Promise<string> {
  const response = await fetch('YOUR_AI_ENDPOINT', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${process.env.YOUR_API_KEY}`,
    },
    body: JSON.stringify({
      messages: messages,
      // Your custom parameters
    }),
  });

  const data = await response.json();
  return data.response; // Adjust based on your API response format
}
```

Options:
- **Ollama** (local LLMs) - Free, runs on your machine
- **Together.ai** - Affordable hosted models
- **Groq** - Fast inference
- **Azure OpenAI** - Enterprise option
- **Custom fine-tuned models** - For specialized knowledge

---

## System Prompt

The AI is given detailed context about Percolator in the system prompt:

```
You are an AI assistant for Percolator, a decentralized perpetual 
futures exchange on Solana.

PLATFORM KNOWLEDGE:
- Uses on-chain orderbooks (slabs)
- Supports SOL/USDC, ETH/USDC, BTC/USDC
- All trades are fully on-chain

USER CONTEXT:
- Wallet Connected: YES/NO
- Wallet Address: [if connected]
- Recent Commands: [last 5]

YOUR ROLE:
- Help users understand the CLI
- Explain trading concepts
- Troubleshoot issues
- Provide actionable guidance
```

**Full prompt:** See `buildSystemPrompt()` in `website/api/src/routes/ai.ts`

---

## Example Interactions

### Example 1: Context-Aware Help

**User:** "How do I place a trade?"

**AI sees:**
- Wallet: ✓ Connected
- Recent: `$ portfolio` → "No portfolio found"

**AI responds:**
```
✓ Your wallet is connected (BWFTK...)

To place a trade:
Use: "trade buy SOL 0.1" or "trade sell ETH 0.5"

⚠️ I notice you haven't initialized your portfolio yet.

First, run:
1. "deposit 0.1" to fund your portfolio
2. Then you can trade!
```

---

### Example 2: Error Troubleshooting

**User:** "Why is my trade failing?"

**AI sees:**
- Recent error: "Error: Price not aligned to tick size"

**AI responds:**
```
I see you had an error:
"Error: Price not aligned to tick size"

This means your price doesn't match the market's tick size.

For SOL/USDC:
- Tick size: $0.01
- Valid prices: $100.00, $100.01, $100.02
- Invalid: $100.015

Try: "markets" to see tick/lot sizes for each pair
```

---

## Features

### ✅ Current Features

- **Context-Aware Responses** - Uses wallet status, command history
- **Command Help** - Explains how to use CLI commands
- **Concept Explanations** - Explains slabs, tick size, lot size, etc.
- **Troubleshooting** - Detects errors in history and helps fix them
- **Conversational** - Maintains chat history for follow-up questions
- **Status Indicators** - Shows "Ready" or "Thinking..." states
- **Copy/Paste Support** - Users can copy chat history

### 🚀 Future Enhancements

1. **Real-Time Portfolio Data**
   - Fetch actual portfolio balance
   - Show open positions
   - Display PnL

2. **Market Data Integration**
   - Current prices from oracle
   - Orderbook liquidity
   - Recent trades

3. **Command Execution**
   - AI can suggest + run commands
   - "Run this for me?" button

4. **Advanced Context**
   - Gas fee estimation
   - Slippage calculation
   - Risk warnings

5. **Multi-Language Support**
   - Spanish, Chinese, Japanese, etc.

---

## Testing

### Test the AI

1. **Start frontend:**
   ```bash
   cd website/frontend
   npm run dev
   ```

2. **Go to:** http://localhost:3000/cli

3. **Try these questions:**
   - "How do I check my portfolio?"
   - "What is a slab?"
   - "Help me place a trade"
   - "Why is my transaction failing?"
   - "Explain tick size"
   - "What commands are available?"

---

## API Endpoint

**Endpoint:** `POST /api/ai/chat`

**Request:**
```json
{
  "question": "How do I deposit funds?",
  "context": {
    "wallet": {
      "connected": true,
      "address": "BWFTKUYSPo3vKPAY9wSbAnH15wH4JyapTE4iA7Rs4MBV"
    },
    "cliHistory": ["$ help", "Available commands..."],
    "availableCommands": ["help", "portfolio", "balance", ...],
    "timestamp": "2025-11-02T10:30:00.000Z"
  },
  "chatHistory": [
    { "role": "user", "text": "What is a slab?" },
    { "role": "assistant", "text": "A slab is..." }
  ]
}
```

**Response:**
```json
{
  "success": true,
  "response": "To deposit funds:\n\n✓ Your wallet is connected...",
  "timestamp": "2025-11-02T10:30:01.000Z"
}
```

---

## Architecture

```
┌─────────────────────────────────────────┐
│         CLI Page (Frontend)             │
│  - Collects context (wallet, history)  │
│  - Sends to AI                          │
│  - Displays response                    │
└─────────────────┬───────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│        AI Route (Backend API)           │
│  - Builds system prompt with context   │
│  - Formats conversation history        │
│  - Calls AI service (OpenAI/Claude)    │
└─────────────────┬───────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│         AI Service (External)           │
│  - OpenAI GPT-4 / GPT-3.5              │
│  - Anthropic Claude                     │
│  - Custom / Self-hosted                 │
└─────────────────────────────────────────┘
```

---

## Security Notes

⚠️ **Important:**
- **Never expose API keys** in frontend code
- Always call AI services from **backend only**
- Validate and sanitize user input
- Rate limit API requests
- Monitor costs (AI APIs charge per token)

---

## Summary

| Feature | Status |
|---------|--------|
| Context Collection | ✅ Working |
| Rule-Based AI | ✅ Working |
| OpenAI Integration | 🟡 Ready (commented out) |
| Claude Integration | 🟡 Ready (commented out) |
| Backend Endpoint | ✅ Working |
| Chat History | ✅ Working |
| Auto-Scroll | ✅ Working |
| Status Indicators | ✅ Working |

**Next Step:** Uncomment the AI API calls and add your API key to enable real AI! 🚀

