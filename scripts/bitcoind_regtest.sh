#!/usr/bin/env bash

set -e

# Configuration
BITCOIN_IMAGE="bitcoin/bitcoin:28.0"
CONTAINER_NAME="bitcoin-regtest"
NETWORK_NAME="fiamma-bitvm-network"
RPC_PORT=18443
RPC_USER="test"
RPC_PASSWORD="1234"
BLOCK_INTERVAL=200
USER_ADDRESS="bcrt1phcnl4zcl2fu047pv4wx6y058v8u0n02at6lthvm7pcf2wrvjm5tqatn90k" #addresses[0]
PEGIN_OPERATOR_ADDRESS="bcrt1pmdx8nnpllj3x750zzfqmjvedv34swuka06vda8qau6csnyx2hq9s6p89qf" #addresses[1]
CHALLENGER_ADDRESS="bcrt1pcq4l9affnwx6xj733syn7alwqfg2u6auvyxptptrz9727slv52wqjvcrgc" #addresses[2]
COMMITTEE_ADDRESS="bcrt1pzvm4mzqjld5quwmmampte0rzcdx9d9rju0h2k43x8aq44mh4937s6cvsrs" #addresses[3]  
PEG_OUT_OPERATOR_ADDRESS="bcrt1pje4hwks7uppkzhvumrezeg6w490v890edj8gal98amfdeu0lhmwsxc6f5g" #addresses[4]

# Bitcoin CLI helpers
bt() {
    docker exec $CONTAINER_NAME bitcoin-cli -regtest -rpcport=$RPC_PORT -rpcuser=$RPC_USER -rpcpassword=$RPC_PASSWORD "$@"
}

bt-benefactor() {
    docker exec $CONTAINER_NAME bitcoin-cli -regtest -rpcwallet=benefactor -rpcport=$RPC_PORT -rpcuser=$RPC_USER -rpcpassword=$RPC_PASSWORD "$@"
}

bt-beneficiary() {
    docker exec $CONTAINER_NAME bitcoin-cli -regtest -rpcwallet=beneficiary -rpcport=$RPC_PORT -rpcuser=$RPC_USER -rpcpassword=$RPC_PASSWORD "$@"
}

# Create mining script
create_mining_script() {
    cat > /tmp/mine_blocks.sh << 'EOF'
#!/bin/bash
RPC_PORT=18443
RPC_USER="test"
RPC_PASSWORD="1234"
BLOCK_INTERVAL_LOCAL=$BLOCK_INTERVAL

while true; do
  bitcoin-cli -regtest -rpcport=$RPC_PORT -rpcuser=$RPC_USER -rpcpassword=$RPC_PASSWORD generatetoaddress 1 "$1"
  sleep $BLOCK_INTERVAL_LOCAL
done
EOF
    chmod +x /tmp/mine_blocks.sh
}

# Setup wallets and mining
setup_wallets() {
    echo "Creating wallets..."
    
    # Create new wallets
    bt -named createwallet wallet_name=benefactor blank=true
    bt -named createwallet wallet_name=beneficiary blank=true
    echo "Wallets 'benefactor' and 'beneficiary' created successfully."

    # Import descriptors for benefactor wallet
    echo "Importing descriptors into 'benefactor' wallet..."
    bt-benefactor importdescriptors '[{
        "desc": "tr(tprv8ZgxMBicQKsPd4arFr7sKjSnKFDVMR2JHw9Y8L9nXN4kiok4u28LpHijEudH3mMYoL4pM5UL9Bgdz2M4Cy8EzfErmU9m86ZTw6hCzvFeTg7/86'\''/1'\''/0'\''/1/*)#jzyeered",
        "active": true,
        "timestamp": "now",
        "internal": true
      }, {
        "desc": "tr(tprv8ZgxMBicQKsPd4arFr7sKjSnKFDVMR2JHw9Y8L9nXN4kiok4u28LpHijEudH3mMYoL4pM5UL9Bgdz2M4Cy8EzfErmU9m86ZTw6hCzvFeTg7/86'\''/1'\''/0'\''/0/*)#rkpcykf4",
        "active": true,
        "timestamp": "now"
    }]'
    echo "Descriptors imported into 'benefactor' wallet."

    # Import descriptors for beneficiary wallet
    echo "Importing descriptors into 'beneficiary' wallet..."
    bt-beneficiary importdescriptors '[{
        "desc": "tr(tprv8ZgxMBicQKsPe72C5c3cugP8b7AzEuNjP4NSC17Dkpqk5kaAmsL6FHwPsVxPpURVqbNwdLAbNqi8Cvdq6nycDwYdKHDjDRYcsMzfshimAUq/86'\''/1'\''/0'\''/1/*)#w4ehwx46",
        "active": true,
        "timestamp": "now",
        "internal": true
      }, {
        "desc": "tr(tprv8ZgxMBicQKsPe72C5c3cugP8b7AzEuNjP4NSC17Dkpqk5kaAmsL6FHwPsVxPpURVqbNwdLAbNqi8Cvdq6nycDwYdKHDjDRYcsMzfshimAUq/86'\''/1'\''/0'\''/0/*)#lpuknn9z",
        "active": true,
        "timestamp": "now"
    }]'
    echo "Descriptors imported into 'beneficiary' wallet."

    # Generate addresses and setup mining
    echo "Generating 6 addresses..."
    addresses=()
    for ((i=0; i < 6; i++)); do
        address=$(bt-benefactor getnewaddress "" bech32m | tr -d '\r')
        addresses+=("$address")
        echo "Generated address $i: $address"
    done

    echo "Addresses: ${addresses[@]}"

    # Mine initial blocks
    echo "Mining 200 blocks to ${addresses[5]}..."
    bt generatetoaddress 200 ${addresses[5]}
    bt-benefactor listaddressgroupings

    # Create UTXOs
    echo "Creating 10 UTXOs for all address..."
    for i in {1..10}; do
        bt-benefactor -named sendtoaddress address=${addresses[0]} amount=0.1 fee_rate=25 subtractfeefromamount=false replaceable=true
        bt-benefactor -named sendtoaddress address=${addresses[1]} amount=0.00005 fee_rate=25 subtractfeefromamount=false replaceable=true
        bt-benefactor -named sendtoaddress address=${addresses[2]} amount=1 fee_rate=25 subtractfeefromamount=false replaceable=true
        bt-benefactor -named sendtoaddress address=${addresses[4]} amount=0.1 fee_rate=25 subtractfeefromamount=false replaceable=true
    done


    # Setup continuous mining
#    create_mining_script
#    docker cp /tmp/mine_blocks.sh $CONTAINER_NAME:/home/bitcoin/mine_blocks.sh
#    docker exec $CONTAINER_NAME chmod +x /home/bitcoin/mine_blocks.sh
#    docker exec -d $CONTAINER_NAME /home/bitcoin/mine_blocks.sh "${addresses[5]}"
}

# Show usage
show_usage() {
    echo "📋 Usage: $0 [start|stop]"
    echo "  start    Start Bitcoin Core in regtest mode"
    echo "  stop     Stop Bitcoin Core and clean up data"
}

# Check Bitcoin Core status
check_bitcoind_status() {
    if docker ps | grep -q $CONTAINER_NAME; then
        echo "✅ Bitcoin Core started successfully"
        echo "🔌 Bitcoin Core RPC is accessible at localhost:${RPC_PORT}"
        return 0
    else
        echo "❌ Failed to start Bitcoin Core"
        return 1
    fi
}

# Clean up data directories
cleanup_data_dirs() {
    echo "🧹 Cleaning up Bitcoin Core data..."
    sudo rm -rf "${PROJECT_ROOT}/bitcoin/regtest"
}

# Stop Bitcoin Core and clean up
stop_bitcoind() {
    if [ "$(docker ps -q -f name=$CONTAINER_NAME)" ]; then
        echo "🛑 Stopping Bitcoin Core..."
        docker stop $CONTAINER_NAME >/dev/null 2>&1
    else
        echo "ℹ️ Bitcoin Core not running"
    fi

    if [ "$(docker ps -aq -f name=$CONTAINER_NAME)" ]; then
        echo "🗑️ Removing Bitcoin Core container..."
        docker rm -vf $CONTAINER_NAME >/dev/null 2>&1
    fi
}

# Start Bitcoin Core
start_bitcoind() {
    # Check network
    if ! docker network inspect $NETWORK_NAME >/dev/null 2>&1; then
        echo "🌐 Creating Docker network..."
        docker network create $NETWORK_NAME
    fi

    # Remove existing container
    stop_bitcoind

    echo "Starting Bitcoin node..."
    docker run --rm -d \
        --name $CONTAINER_NAME \
        --network $NETWORK_NAME \
        -p $RPC_PORT:$RPC_PORT \
        $BITCOIN_IMAGE \
        -printtoconsole \
        -regtest=1 \
        -rest \
        -rpcbind=0.0.0.0 \
        -rpcallowip=0.0.0.0/0 \
        -rpcport=$RPC_PORT \
        -rpcuser=$RPC_USER \
        -rpcpassword=$RPC_PASSWORD \
        -server \
        -txindex=1 \
        -walletrbf=0 \
        -mempoolfullrbf=0

    echo "Waiting for Bitcoin node to start..."
    sleep 5

    setup_wallets
    echo "Setup complete! Mining process is running in the background inside the container."
}

# Show usage
show_usage() {
    echo "Usage: $0 [start|stop]"
    echo "  start    Start Bitcoin node"
    echo "  stop     Stop Bitcoin node"
}

# Main function
main() {
    case "$1" in
        start)
            start_bitcoind
            ;;
        stop)
            stop_bitcoind
            ;;
        *)
            show_usage
            exit 1
            ;;
    esac
}

# Check command line arguments
if [ $# -ne 1 ]; then
    show_usage
    exit 1
fi

main "$1"

