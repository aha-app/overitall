class Oit < Formula
  desc "TUI combining process management and log viewing"
  homepage "https://github.com/aha-app/overitall"
  version "0.1.59"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/aha-app/overitall/releases/download/v#{version}/oit-macos-arm64.tar.gz"
      sha256 "PLACEHOLDER" # macos-arm64
    else
      url "https://github.com/aha-app/overitall/releases/download/v#{version}/oit-macos-x86_64.tar.gz"
      sha256 "PLACEHOLDER" # macos-x86_64
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/aha-app/overitall/releases/download/v#{version}/oit-linux-arm64.tar.gz"
      sha256 "PLACEHOLDER" # linux-arm64
    else
      url "https://github.com/aha-app/overitall/releases/download/v#{version}/oit-linux-x86_64.tar.gz"
      sha256 "PLACEHOLDER" # linux-x86_64
    end
  end

  def install
    bin.install "oit"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/oit --version")
  end
end
