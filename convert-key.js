const bs58 = require('bs58');
const fs = require('fs');

const base58Key = process.argv[2];
const outputFile = process.argv[3];

try {
  const decoded = bs58.decode(base58Key);
  const keyArray = Array.from(decoded);
  fs.writeFileSync(outputFile, JSON.stringify(keyArray));
  console.log(`✅ Keypair saved to ${outputFile}`);
} catch (e) {
  console.error('Error:', e.message);
}
