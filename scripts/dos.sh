#!/bin/bash

requests=100

if [[ $1 =~ ^[0-9]+$ ]]; then
  requests="$1"
fi

do_request() {
  echo "Request $1..."
  start=$(date +%s)
  result=$(curl -o /dev/null -s -w "%{http_code}" 'http://localhost:4444')
  end=$(date +%s)
  echo "Finished $1; Time $((end - start))s; Result $result"
}

echo "Launching $requests requests"
total_start=$(date +%s)
for ((i = 0; i < $requests; i++)); do
  do_request $i &
done

wait
total_end=$(date +%s)

echo "--------------------"
echo "Requests $requests"
echo "Total run time $((total_end - total_start))s"
