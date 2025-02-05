module hanamictl

go 1.22.5

replace github.com/kitsudaiki/OpenHanami => ../../sdk/go/hanami_sdk

require (
	github.com/kitsudaiki/OpenHanami v0.3.1-0.20230916184520-abb03e487e58
	github.com/olekukonko/tablewriter v0.0.5
	github.com/spf13/cobra v1.8.1
	golang.org/x/term v0.29.0
)

require (
	github.com/golang/protobuf v1.5.3 // indirect
	github.com/gorilla/websocket v1.5.3 // indirect
	github.com/inconshreveable/mousetrap v1.1.0 // indirect
	github.com/mattn/go-runewidth v0.0.9 // indirect
	github.com/spf13/pflag v1.0.5 // indirect
	golang.org/x/sys v0.30.0 // indirect
	google.golang.org/protobuf v1.33.0 // indirect
)
