# run cargo run -p goose-server: https://github.com/block/goose/pull/237

curl -N http://localhost:3000/reply \
   -H "Content-Type: application/json" \
   -H "Accept: text/event-stream" \
   -H "x-protocol: data" \
   -d @test.json