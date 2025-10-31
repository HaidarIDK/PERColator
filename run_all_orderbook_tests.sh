#!/bin/bash

# ========================================
# All Orderbook Tests Runner
# ========================================
#
# Runs all 6 orderbook E2E test suites and reports results.
# Tests all 34 implemented scenarios (100% coverage).
#
# Test suites:
# 1. test_core_scenarios.sh - Core orderbook operations
# 2. test_modify_order.sh - Order modification
# 3. test_orderbook_extended.sh - Advanced order types
# 4. test_matching_engine.sh - IOC/FOK/STP matching
# 5. test_matching_scenarios.sh - Matching scenarios
# 6. test_orderbook_comprehensive.sh - Edge cases & robustness

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Results tracking
TOTAL_TESTS=6
PASSED_TESTS=0
FAILED_TESTS=0
declare -a FAILED_TEST_NAMES

echo "========================================"
echo "  Orderbook Test Suite Runner"
echo "========================================"
echo ""
echo "Running all 6 orderbook E2E tests..."
echo "Testing 34/40 scenarios (100% of implemented features)"
echo ""

# Test 1: Core scenarios
echo -e "${CYAN}[1/6] Running test_core_scenarios.sh...${NC}"
if ./test_core_scenarios.sh > /tmp/test_core.log 2>&1; then
    echo -e "${GREEN}✓ test_core_scenarios.sh PASSED${NC}"
    echo "  Scenarios: 1, 2, 5, 18, 24, 28"
    ((PASSED_TESTS++))
else
    echo -e "${RED}✗ test_core_scenarios.sh FAILED${NC}"
    FAILED_TEST_NAMES+=("test_core_scenarios.sh")
    ((FAILED_TESTS++))
fi
echo ""

# Test 2: Modify order
echo -e "${CYAN}[2/6] Running test_modify_order.sh...${NC}"
if ./test_modify_order.sh > /tmp/test_modify.log 2>&1; then
    echo -e "${GREEN}✓ test_modify_order.sh PASSED${NC}"
    echo "  Scenarios: 6, 7, 31, 32"
    ((PASSED_TESTS++))
else
    echo -e "${RED}✗ test_modify_order.sh FAILED${NC}"
    FAILED_TEST_NAMES+=("test_modify_order.sh")
    ((FAILED_TESTS++))
fi
echo ""

# Test 3: Extended order types
echo -e "${CYAN}[3/6] Running test_orderbook_extended.sh...${NC}"
if ./test_orderbook_extended.sh > /tmp/test_extended.log 2>&1; then
    echo -e "${GREEN}✓ test_orderbook_extended.sh PASSED${NC}"
    echo "  Scenarios: 8, 9, 12, 15, 16"
    ((PASSED_TESTS++))
else
    echo -e "${RED}✗ test_orderbook_extended.sh FAILED${NC}"
    FAILED_TEST_NAMES+=("test_orderbook_extended.sh")
    ((FAILED_TESTS++))
fi
echo ""

# Test 4: Matching engine
echo -e "${CYAN}[4/6] Running test_matching_engine.sh...${NC}"
if ./test_matching_engine.sh > /tmp/test_matching_engine.log 2>&1; then
    echo -e "${GREEN}✓ test_matching_engine.sh PASSED${NC}"
    echo "  Scenarios: 10, 11, 13, 14, 26"
    ((PASSED_TESTS++))
else
    echo -e "${RED}✗ test_matching_engine.sh FAILED${NC}"
    FAILED_TEST_NAMES+=("test_matching_engine.sh")
    ((FAILED_TESTS++))
fi
echo ""

# Test 5: Matching scenarios
echo -e "${CYAN}[5/6] Running test_matching_scenarios.sh...${NC}"
if ./test_matching_scenarios.sh > /tmp/test_matching_scenarios.log 2>&1; then
    echo -e "${GREEN}✓ test_matching_scenarios.sh PASSED${NC}"
    echo "  Scenarios: 3, 4, 19, 20, 27, 29, 33"
    ((PASSED_TESTS++))
else
    echo -e "${RED}✗ test_matching_scenarios.sh FAILED${NC}"
    FAILED_TEST_NAMES+=("test_matching_scenarios.sh")
    ((FAILED_TESTS++))
fi
echo ""

# Test 6: Comprehensive edge cases
echo -e "${CYAN}[6/6] Running test_orderbook_comprehensive.sh...${NC}"
if ./test_orderbook_comprehensive.sh > /tmp/test_comprehensive.log 2>&1; then
    echo -e "${GREEN}✓ test_orderbook_comprehensive.sh PASSED${NC}"
    echo "  Scenarios: 22, 23, 25, 30, 34, 38, 39"
    ((PASSED_TESTS++))
else
    echo -e "${RED}✗ test_orderbook_comprehensive.sh FAILED${NC}"
    FAILED_TEST_NAMES+=("test_orderbook_comprehensive.sh")
    ((FAILED_TESTS++))
fi
echo ""

# Results summary
echo "========================================"
echo "  Test Results Summary"
echo "========================================"
echo ""
echo -e "Total tests run: ${TOTAL_TESTS}"
echo -e "${GREEN}Passed: ${PASSED_TESTS}${NC}"
echo -e "${RED}Failed: ${FAILED_TESTS}${NC}"
echo ""

if [ $FAILED_TESTS -eq 0 ]; then
    echo "========================================"
    echo -e "  ${GREEN}✓ ALL TESTS PASSED! ✓${NC}"
    echo "========================================"
    echo ""
    echo "Test Coverage:"
    echo "  • 34/34 implemented scenarios tested (100%)"
    echo "  • 34/40 total scenarios implemented (85%)"
    echo ""
    echo "Scenario Categories:"
    echo "  ✓ Core Order Book (11 scenarios)"
    echo "  ✓ Advanced Order Types (7 scenarios)"
    echo "  ✓ Risk Controls (5 scenarios)"
    echo "  ✓ Matching Engine (6 scenarios)"
    echo "  ✓ Edge Cases & Robustness (5 scenarios)"
    echo ""
    echo "Test logs available in /tmp/test_*.log"
    echo ""
    exit 0
else
    echo "========================================"
    echo -e "  ${RED}✗ SOME TESTS FAILED ✗${NC}"
    echo "========================================"
    echo ""
    echo "Failed tests:"
    for test_name in "${FAILED_TEST_NAMES[@]}"; do
        echo -e "  ${RED}✗ $test_name${NC}"
    done
    echo ""
    echo "Check logs in /tmp/test_*.log for details"
    echo ""
    exit 1
fi
