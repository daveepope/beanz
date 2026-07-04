class Beanz < Formula
  desc "Cognitive debt scoring for AI coding sessions"
  homepage "https://github.com/daveepope/beanz"
  license "MIT"
  version "0.1.0"

  head do
    url "https://github.com/daveepope/beanz.git", branch: "main"
    depends_on "rust" => :build
  end

  on_macos do
    on_arm do
      url "https://github.com/daveepope/beanz/releases/download/v0.1.0/beanz-v0.1.0-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_AARCH64_APPLE_DARWIN"
    end
    on_intel do
      url "https://github.com/daveepope/beanz/releases/download/v0.1.0/beanz-v0.1.0-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_X86_64_APPLE_DARWIN"
    end
  end

  on_linux do
    url "https://github.com/daveepope/beanz/releases/download/v0.1.0/beanz-v0.1.0-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "PLACEHOLDER_X86_64_UNKNOWN_LINUX_GNU"
  end

  def install
    if build.head?
      system "cargo", "install", *std_cargo_args
    else
      bin.install "beanz"
    end
  end

  test do
    assert_match "usage:", shell_output("#{bin}/beanz --foo 2>&1", 2)
  end
end
