class Oit < Formula
  desc "TUI combining process management and log viewing"
  homepage "https://github.com/aha-app/overitall"
  version "0.3.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/aha-app/overitall/releases/download/v#{version}/oit-macos-arm64.tar.gz"
      sha256 "06471680f658b1deeb350ffce4019f5ceb7fa877f0eb9aa6356492eff5ad2cf4" # macos-arm64
    else
      url "https://github.com/aha-app/overitall/releases/download/v#{version}/oit-macos-x86_64.tar.gz"
      sha256 "db13f5387a70367620a3818383681ed0c1f58a5320ec3259310795a39f28abc5" # macos-x86_64
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/aha-app/overitall/releases/download/v#{version}/oit-linux-arm64.tar.gz"
      sha256 "f81df9ac529d0a6a35252ba17d7a9b3a76a22dba0149595dd049f0c5391ecf0a" # linux-arm64
    else
      url "https://github.com/aha-app/overitall/releases/download/v#{version}/oit-linux-x86_64.tar.gz"
      sha256 "b9f0dce42209197d9388c9e75297d758c8b2c1dc365d2e622e1a7a0beec03faf" # linux-x86_64
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
