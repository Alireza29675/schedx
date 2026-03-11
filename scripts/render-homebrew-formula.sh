#!/bin/sh
set -eu

if [ "$#" -ne 2 ]; then
    echo "Usage: $0 <version> <sha256>" >&2
    exit 1
fi

version="$1"
sha256="$2"

cat <<EOF
class Schedx < Formula
  desc "Secure and reliable scheduler CLI for commands, prompts, and webhooks"
  homepage "https://github.com/Alireza29675/schedx"
  url "https://github.com/Alireza29675/schedx/archive/refs/tags/v${version}.tar.gz"
  sha256 "${sha256}"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: ".")
  end

  test do
    assert_equal "[]\n", shell_output("#{bin}/schedx list --json")
  end
end
EOF
