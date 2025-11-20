// Vercel serverless function entry point
const app = require('../dist/index.js');

// Export the default export (the Express app)
module.exports = app.default || app;