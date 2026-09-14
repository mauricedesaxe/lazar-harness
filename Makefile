.PHONY: build test vet fmt clean

build:
	go build -o harness-check ./cmd/harness-check

test:
	go test ./...

vet:
	go vet ./...

fmt:
	gofmt -w cmd internal

clean:
	rm -f harness-check
