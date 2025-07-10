DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
ROOT=$DIR/..

solana-test-validator \
  --log \
  --rpc-port 7799 \
  -r
