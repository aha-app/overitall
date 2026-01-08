class Oit < Formula
  desc "TUI combining process management and log viewing"
  homepage "https://github.com/aha-app/overitall"
  version "0.1.62"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/aha-app/overitall/releases/download/v#{version}/oit-macos-arm64.tar.gz"
      sha256 "14f15ea357c422862e2ed9b49cc22d9f0fc9028f5cb4e7ad43344f3d72a00460" # macos-arm64
    else
      url "https://github.com/aha-app/overitall/releases/download/v#{version}/oit-macos-x86_64.tar.gz"
      sha256 "387d96371abba88de7f9362f0e64ef8afe37c4311c6838fc91c482afdf241110" # macos-x86_64
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/aha-app/overitall/releases/download/v#{version}/oit-linux-arm64.tar.gz"
      sha256 "bdf40f5c8609656ed0bf75260b363208d928a0be9be473dad5f0d7174f8e767f" # linux-arm64
    else
      url "https://github.com/aha-app/overitall/releases/download/v#{version}/oit-linux-x86_64.tar.gz"
      sha256 "4f1f1b3c0cfa9544a246233fc174682cad894465c1ec8dfdb4eb351b3d92b460" # linux-x86_64
    end
  end

  def install
    bin.install "oit"
    man1.install "oit.1"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/oit --version")
  end
end
