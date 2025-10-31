#!/usr/bin/env bash
#
# Comprehensive Test Runner
#
# Runs all functional CLI tests for the percolator exchange
#

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${YELLOW}======================================${NC}"
echo -e "${YELLOW}  Percolator CLI Test Suite${NC}"
echo -e "${YELLOW}======================================${NC}"
echo ""

# Test counters
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# Test result tracking
declare -a PASSED_LIST
declare -a FAILED_LIST

# Function to run a test
run_test() {
    local test_name=$1
    local test_script=$2

    ((TOTAL_TESTS++))
    echo -e "${YELLOW}Running: ${test_name}...${NC}"

    if $test_script > /tmp/${test_name}.log 2>&1; then
        echo -e "${GREEN}✓ PASSED${NC}"
        ((PASSED_TESTS++))
        PASSED_LIST+=("$test_name")
    else
        echo -e "${RED}✗ FAILED${NC}"
        ((FAILED_TESTS++))
        FAILED_LIST+=("$test_name")
        echo "  Log: /tmp/${test_name}.log"
    fi
    echo ""
}

echo -e "${YELLOW}=== Core Orderbook Tests (6) ===${NC}"
echo ""

run_test "test_core_scenarios" ./test_core_scenarios.sh
run_test "test_modify_order" ./test_modify_order.sh
run_test "test_orderbook_extended" ./test_orderbook_extended.sh
run_test "test_matching_engine" ./test_matching_engine.sh
run_test "test_matching_scenarios" ./test_matching_scenarios.sh
run_test "test_orderbook_comprehensive" ./test_orderbook_comprehensive.sh

echo -e "${YELLOW}=== Additional Orderbook Tests (3) ===${NC}"
echo ""

run_test "test_halt_resume" ./test_halt_resume.sh
run_test "test_orderbook_simple" ./test_orderbook_simple.sh
run_test "test_orderbook_working" ./test_orderbook_working.sh

echo -e "${YELLOW}=== Funding Tests (2) ===${NC}"
echo ""

run_test "test_funding_simple" ./test_funding_simple.sh
run_test "test_funding_working" ./test_funding_working.sh

echo ""
echo -e "${YELLOW}======================================${NC}"
echo -e "${YELLOW}  Test Summary${NC}"
echo -e "${YELLOW}======================================${NC}"
echo ""
echo "Total Tests:  $TOTAL_TESTS"
echo -e "${GREEN}Passed:       $PASSED_TESTS${NC}"
if [ $FAILED_TESTS -gt 0 ]; then
    echo -e "${RED}Failed:       $FAILED_TESTS${NC}"
else
    echo "Failed:       0"
fi
echo ""

if [ $FAILED_TESTS -eq 0 ]; then
    echo -e "${GREEN}======================================${NC}"
    echo -e "${GREEN}  ALL TESTS PASSED! ✓${NC}"
    echo -e "${GREEN}======================================${NC}"
    echo ""
    echo "Test Coverage:"
    echo "  ✓ Core orderbook operations (6 test suites)"
    echo "  ✓ Additional orderbook scenarios (3 test suites)"
    echo "  ✓ Funding mechanics (2 test suites)"
    echo ""
    echo "Total: 11/11 tests passing (100%)"
    exit 0
else
    echo -e "${RED}======================================${NC}"
    echo -e "${RED}  SOME TESTS FAILED${NC}"
    echo -e "${RED}======================================${NC}"
    echo ""

    if [ ${#PASSED_LIST[@]} -gt 0 ]; then
        echo -e "${GREEN}Passed tests:${NC}"
        for test in "${PASSED_LIST[@]}"; do
            echo "  ✓ $test"
        done
        echo ""
    fi

    if [ ${#FAILED_LIST[@]} -gt 0 ]; then
        echo -e "${RED}Failed tests:${NC}"
        for test in "${FAILED_LIST[@]}"; do
            echo "  ✗ $test (log: /tmp/${test}.log)"
        done
        echo ""
    fi

    exit 1
fi
