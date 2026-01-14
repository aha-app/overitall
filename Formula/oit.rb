class Oit < Formula
  desc "TUI combining process management and log viewing"
  homepage "https://github.com/aha-app/overitall"
  version "0.1.63"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/aha-app/overitall/releases/download/v#{version}/oit-macos-arm64.tar.gz"
      sha256 "b35112c9cbdc1c11c478e463a11f481b4e12316c4ac7e024aed8941dc9f64bde" # macos-arm64
    else
      url "https://github.com/aha-app/overitall/releases/download/v#{version}/oit-macos-x86_64.tar.gz"
      sha256 "b6e652eba943167a05d1ebdd43319df74a673e31bde939c72e7edd9cf067f349" # macos-x86_64
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/aha-app/overitall/releases/download/v#{version}/oit-linux-arm64.tar.gz"
      sha256 "d407613d72778786d90f56cf37b4a7d2c630f7afc20fe764bd1e68dbec23c82b" # linux-arm64
    else
      url "https://github.com/aha-app/overitall/releases/download/v#{version}/oit-linux-x86_64.tar.gz"
      sha256 "714f54031f58a89e4c43ac3f11d0f40cdc502cafd500981824be2fb9dc8d6f1d" # linux-x86_64
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
