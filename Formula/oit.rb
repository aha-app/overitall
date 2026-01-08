class Oit < Formula
  desc "TUI combining process management and log viewing"
  homepage "https://github.com/aha-app/overitall"
  version "0.1.61"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/aha-app/overitall/releases/download/v#{version}/oit-macos-arm64.tar.gz"
      sha256 "44a39d3b47f0bf46b5149419aad3eb085d9591a45cd813d9ac8ac1ca797a9f1a" # macos-arm64
    else
      url "https://github.com/aha-app/overitall/releases/download/v#{version}/oit-macos-x86_64.tar.gz"
      sha256 "b33e335936cf65322918b5b6752888311c26e42213e8b52976b5ab1e20511dbe" # macos-x86_64
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/aha-app/overitall/releases/download/v#{version}/oit-linux-arm64.tar.gz"
      sha256 "cbec6a87c0f82151ee0dfd76d5bf1b4613b1e80a3849bc2a5053732e76005881" # linux-arm64
    else
      url "https://github.com/aha-app/overitall/releases/download/v#{version}/oit-linux-x86_64.tar.gz"
      sha256 "752eeedcb5c3a7615af139b8b135ffc7e7eef319b72d49bff4d3098adccec433" # linux-x86_64
    end
  end

  def install
    bin.install "oit"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/oit --version")
  end
end
