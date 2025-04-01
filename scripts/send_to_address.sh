count=100
for ((i=1; i<=$count; i++)); do
#  echo "这是第 $i 次执行"
  bitcoin-cli -regtest -rpcwallet=benefactor -named sendtoaddress address="bcrt1pmdx8nnpllj3x750zzfqmjvedv34swuka06vda8qau6csnyx2hq9s6p89qf" amount=0.1 fee_rate=25 subtractfeefromamount=false replaceable=true comment="2 pizzas" comment_to="jeremy" verbose=true

  if (( i % 20 == 0 )); then
    echo "这是第 $i 次执行 - gen_regtest_block"
    # gen block
    bash gen_regtest_block.sh
  fi
done
