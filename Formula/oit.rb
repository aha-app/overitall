class Oit < Formula
  desc "TUI combining process management and log viewing"
  homepage "https://github.com/aha-app/overitall"
  version "0.1.64"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/aha-app/overitall/releases/download/v#{version}/oit-macos-arm64.tar.gz"
      sha256 "680cd9425e03fe1e3cbd8bc5d59050d9c813314e87dc93a17553628fa1da3513" # macos-arm64
    else
      url "https://github.com/aha-app/overitall/releases/download/v#{version}/oit-macos-x86_64.tar.gz"
      sha256 "ffb24e448e3db0c95132d2a7a7da379932aad26a1ca45e23be5432a5fae2d6a9" # macos-x86_64
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/aha-app/overitall/releases/download/v#{version}/oit-linux-arm64.tar.gz"
      sha256 "69b7e5e0393e5418a5242a9ec78769e24bc25430f5ea4d6753fc80acbf9f3992" # linux-arm64
    else
      url "https://github.com/aha-app/overitall/releases/download/v#{version}/oit-linux-x86_64.tar.gz"
      sha256 "bd7c253e41add41135158a8df32ad9ff62bdade013c2744780738cb707a8b166" # linux-x86_64
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
