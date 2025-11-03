# Website Folder Cleanup Analysis

## Current State

### File Sizes
- `frontend/src/app/dashboard/page.tsx` - **4,371 lines** (CRITICAL: needs splitting)
- Other pages: Generally reasonable size

### Issues Found

#### 1. Large Dashboard Component (PRIORITY: HIGH)
- **Location**: `frontend/src/app/dashboard/page.tsx` (4,371 lines)
- **Problem**: Contains multiple inline components that should be extracted
- **Components to Extract**:
  - `LightweightChart` (lines ~54-518) - Should be `components/charts/LightweightChart.tsx`
  - `TradingViewChartComponent` (lines ~522-792) - Should be `components/charts/TradingViewChart.tsx`
  - `OrderBook` (lines ~925-1002) - Should be `components/trading/OrderBook.tsx`
  - `CrossSlabTrader` (lines ~2088-3075) - Should be `components/trading/CrossSlabTrader.tsx`
  - `OrderForm` (lines ~3076-3484) - Should be `components/trading/OrderForm.tsx`
  - Main dashboard layout (lines ~3485+) - Should be simplified

#### 2. Console Statements (PRIORITY: MEDIUM)
- **Found**: 108+ `console.log/error/warn` statements
- **Action**: Remove or replace with proper logging utility
- **Files affected**:
  - `dashboard/page.tsx` (majority)
  - `api-client.ts`
  - `trade/page.tsx`
  - Others

#### 3. Build Artifacts (PRIORITY: LOW)
- `node_modules/` folders (should be gitignored - already are)
- `.next/` folder (should be gitignored - already is)
- `api-startup.log` (should be removed)
- Various log files in node_modules

#### 4. Documentation Files (PRIORITY: LOW)
- Multiple `.md` files in root of frontend
- Consider organizing into `docs/` folder:
  - `DASHBOARD_ARCHITECTURE.md`
  - `FRONTEND_FOLDER_DEPENDENCIES.md`
  - `SETUP.md`

#### 5. Unused/Redundant Code
- Need to check for duplicate functionality
- Check for unused imports
- Check for dead code paths

## Recommended Cleanup Steps

### Phase 1: Quick Wins (Remove clutter)
1. Remove log files
2. Ensure .gitignore is comprehensive
3. Remove console.logs or convert to proper logging

### Phase 2: Component Extraction (Most Important)
1. Extract `LightweightChart` component
2. Extract `TradingViewChartComponent` component  
3. Extract `OrderBook` component
4. Extract `CrossSlabTrader` component
5. Extract `OrderForm` component
6. Simplify main dashboard component

### Phase 3: Code Organization
1. Organize documentation files
2. Create proper folder structure for components
3. Consolidate duplicate code
4. Optimize imports

### Phase 4: Performance Optimization
1. Add React.memo where appropriate
2. Implement proper loading states
3. Add error boundaries
4. Optimize re-renders

## Folder Structure Recommendation

```
website/frontend/src/
├── app/                 # Next.js pages
├── components/
│   ├── charts/         # Chart components
│   ├── trading/        # Trading components
│   ├── ui/             # UI primitives
│   └── ...             # Other components
├── lib/                 # Utilities and clients
└── hooks/               # Custom React hooks (to be created)
```

