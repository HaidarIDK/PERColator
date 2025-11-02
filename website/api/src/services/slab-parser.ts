/**
 * Slab Binary Parser
 * 
 * Parses the binary orderbook data from the Solana slab account
 * to extract real bids and asks.
 */

interface Order {
  price: number;
  quantity: number;
  owner?: string;
}

interface ParsedOrderbook {
  bids: Order[];
  asks: Order[];
  markPrice: number;
  tickSize: number;
  lotSize: number;
}

/**
 * Parse the slab account binary data
 */
export function parseSlabOrderbook(data: Buffer): ParsedOrderbook {
  try {
    let offset = 0;
    
    // Skip discriminator (8 bytes)
    offset += 8;
    
    // Read mark price (i64, 8 bytes) - fixed point with 6 decimals
    const markPriceRaw = data.readBigInt64LE(offset);
    const markPrice = Number(markPriceRaw) / 1_000_000;
    offset += 8;
    
    // Read fees (u64, 8 bytes)
    offset += 8;
    
    // Read contract size (u64, 8 bytes)
    offset += 8;
    
    // Skip LP owner (32 bytes)
    offset += 32;
    
    // Skip router ID (32 bytes)
    offset += 32;
    
    // Skip instrument (u32, 4 bytes)
    offset += 4;
    
    // Read tick size (FixedU64 = u64 + i64, 16 bytes)
    const tickSizeRaw = Number(data.readBigUInt64LE(offset));
    const tickSize = tickSizeRaw / 1_000_000;
    offset += 16; // Skip both u64 and i64
    
    // Read lot size (FixedU64 = u64 + i64, 16 bytes)
    const lotSizeRaw = Number(data.readBigUInt64LE(offset));
    const lotSize = lotSizeRaw / 1_000_000;
    offset += 16;
    
    // Skip min order size (u64, 8 bytes)
    offset += 8;
    
    console.log(`📊 Slab Config: Mark=${markPrice}, Tick=${tickSize}, Lot=${lotSize}`);
    console.log(`   Scanning from offset ${offset}, remaining ${data.length - offset} bytes`);
    
    // Now we need to parse the orderbook
    // The slab stores orders in a binary tree structure
    // For simplicity, we'll scan the remaining data for valid orders
    
    const bids: Order[] = [];
    const asks: Order[] = [];
    let scannedEntries = 0;
    let validEntries = 0;
    
    // Scan remaining data for order-like patterns
    const remainingData = data.slice(offset);
    const chunkSize = 64; // Approximate size of an order entry
    
    for (let i = 0; i < remainingData.length - chunkSize; i += 8) {
      try {
        scannedEntries++;
        
        // Try to read a potential price (i64)
        const priceRaw = remainingData.readBigInt64LE(i);
        
        // Sanity check: price should be positive and reasonable
        if (priceRaw <= 0 || priceRaw > 1_000_000_000_000) continue;
        
        const price = Number(priceRaw) / 1_000_000;
        
        // Try to read quantity 8 bytes after price
        const qtyRaw = remainingData.readBigInt64LE(i + 8);
        
        // Sanity check: quantity should be positive and reasonable
        if (qtyRaw <= 0 || qtyRaw > 1_000_000_000_000) continue;
        
        const quantity = Number(qtyRaw) / 1_000_000;
        
        // Check if price is within reasonable range of mark price (±20%)
        const priceDeviation = Math.abs(price - markPrice) / markPrice;
        if (priceDeviation > 0.20) continue;
        
        // Check if quantity is reasonable (> 0 and < 1000 SOL)
        if (quantity <= 0 || quantity > 1000) continue;
        
        validEntries++;
        
        // Log first few valid entries for debugging
        if (validEntries <= 3) {
          console.log(`   Found order: $${price} x ${quantity} SOL (offset ${i})`);
        }
        
        // Determine side based on price relative to mark
        if (price < markPrice) {
          // Bid (buy order below mark)
          bids.push({ price, quantity });
        } else if (price > markPrice) {
          // Ask (sell order above mark)
          asks.push({ price, quantity });
        }
      } catch (e) {
        // Skip invalid entries
        continue;
      }
    }
    
    console.log(`   Scanned ${scannedEntries} entries, found ${validEntries} valid orders`);
    
    // Remove duplicates and sort
    const uniqueBids = Array.from(new Map(bids.map(b => [b.price, b])).values());
    const uniqueAsks = Array.from(new Map(asks.map(a => [a.price, a])).values());
    
    uniqueBids.sort((a, b) => b.price - a.price); // Descending
    uniqueAsks.sort((a, b) => a.price - b.price); // Ascending
    
    console.log(`✅ Parsed ${uniqueBids.length} bids, ${uniqueAsks.length} asks`);
    if (uniqueBids.length > 0) console.log(`   Best bid: $${uniqueBids[0].price} x ${uniqueBids[0].quantity}`);
    if (uniqueAsks.length > 0) console.log(`   Best ask: $${uniqueAsks[0].price} x ${uniqueAsks[0].quantity}`);
    
    return {
      bids: uniqueBids.slice(0, 20), // Top 20 bids
      asks: uniqueAsks.slice(0, 20), // Top 20 asks
      markPrice,
      tickSize,
      lotSize,
    };
  } catch (error) {
    console.error('❌ Failed to parse slab:', error);
    throw error;
  }
}

/**
 * Advanced parser using tree structure knowledge
 * (TODO: Implement if we understand the exact tree layout)
 */
export function parseSlabOrderbookAdvanced(data: Buffer): ParsedOrderbook {
  // This would parse the actual binary tree structure
  // For now, use the heuristic parser above
  return parseSlabOrderbook(data);
}

