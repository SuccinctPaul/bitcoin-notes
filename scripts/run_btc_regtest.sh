# Ref: https://github.com/SuccinctPaul/scripts-box/blob/35600dc45979a18cb82da991119d2b9326d28c93/bitcoin/run_btc_regtest.sh
docker run \
  -v ~/.bitcoin/data:/home/bitcoin/.bitcoin \
  -d \
  --name bitcoin-regtest \
  -p 18443:18443 \
  -p 18444:18444 \
  bitcoin/bitcoin:28.0 \
  -printtoconsole \
  -regtest=1 \
  -rest \
  -rpcbind=0.0.0.0 \
  -rpcallowip=0.0.0.0/0 \
  -rpcport=18443 \
  -rpcuser=username \
  -rpcpassword=userpswd \
  -server \
  -txindex=1 \
#  -walletrbf=0 \
#  -mempoolfullrbf=0 \
  -rpcauth='username:5cab6e9e4fe9282621ef9d351c0710b7$4a2f92ba573a0f79085013ebee0fbf48e1567428c572a5642afdbe4836232b70'
