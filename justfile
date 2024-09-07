_default:
  @just --list

now := `date "+%FT%T"`

server:
  RUST_LOG=debug cargo r -- server --ntfy-topic emctoo-hello

client:
  RUST_LOG=debug cargo r -- client 

copy content=now:
  RUST_LOG=debug cargo r -- copy {{content}}

paste:
  RUST_LOG=debug cargo r -- paste
