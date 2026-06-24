class Oit < Formula
  desc "TUI combining process management and log viewing"
  homepage "https://github.com/aha-app/overitall"
  version "1.0.4"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/aha-app/overitall/releases/download/v#{version}/oit-macos-arm64.tar.gz"
      sha256 "204bdf219078f1649ffa1cc1e5b26f192e784f5b54ebf759285688a2b1bddd7d" # macos-arm64
    else
      url "https://github.com/aha-app/overitall/releases/download/v#{version}/oit-macos-x86_64.tar.gz"
      sha256 "8e093371b35088f8d642af3b3680f36a28adbdd2dd384f21032048ed71b94a21" # macos-x86_64
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/aha-app/overitall/releases/download/v#{version}/oit-linux-arm64.tar.gz"
      sha256 "3e38e1ded82f394e3cf13eb20d6225d076e14ef19148d2d153d7c676e1e96d1f" # linux-arm64
    else
      url "https://github.com/aha-app/overitall/releases/download/v#{version}/oit-linux-x86_64.tar.gz"
      sha256 "89726a919c08b3cbc79317b30a5489471b4737386a683bdf5215e0d75e399df3" # linux-x86_64
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
