class Oit < Formula
  desc "TUI combining process management and log viewing"
  homepage "https://github.com/aha-app/overitall"
  version "0.1.60"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/aha-app/overitall/releases/download/v#{version}/oit-macos-arm64.tar.gz"
      sha256 "dc3a6323c588a6a1a2198b90f5f3d43d0e77a78d9385947bdc9deddf5e7ce081" # macos-arm64
    else
      url "https://github.com/aha-app/overitall/releases/download/v#{version}/oit-macos-x86_64.tar.gz"
      sha256 "d8d07a131a06363e4848468ada8295575e553c20f450112d25ce9d8c367ee9a6" # macos-x86_64
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/aha-app/overitall/releases/download/v#{version}/oit-linux-arm64.tar.gz"
      sha256 "fdd1d3305b2d0e77a9ebf723cfd58da01563fc894f17537863716a7d0f1060d6" # linux-arm64
    else
      url "https://github.com/aha-app/overitall/releases/download/v#{version}/oit-linux-x86_64.tar.gz"
      sha256 "225ce13791933a2edd112d81da9c376aaa3dbe402884c6fec2feaca8a1506eab" # linux-x86_64
    end
  end

  def install
    bin.install "oit"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/oit --version")
  end
end
