# go-sdk

## prepare protobuf-message

```
cd ainari_sdk
protoc --go_out=. --proto_path ../../../libs/protobuf ainari_messages.proto3
sed -i 's/ainari_messages/ainari_sdk/g' ainari_messages.proto3.pb.go
```
