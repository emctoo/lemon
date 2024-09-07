_default:
  @just --list

now := `date "+%FT%T"`

hub:
  RUST_LOG=debug cargo r -- hub --ntfy-topic emctoo-hello

server:
  RUST_LOG=debug cargo r -- server

client:
  RUST_LOG=debug cargo r -- client 

copy content=now:
  RUST_LOG=debug cargo r -- copy --host 192.168.8.34 {{content}}

paste:
  RUST_LOG=debug cargo r -- paste
