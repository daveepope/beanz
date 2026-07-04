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
      sha256 "5b951533bffd63916faf20d6a51e149614a318ffe8b30f4b6063aa116825b28c"
    end
    on_intel do
      url "https://github.com/daveepope/beanz/releases/download/v0.1.0/beanz-v0.1.0-x86_64-apple-darwin.tar.gz"
      sha256 "d988dabd1d9fde4a09fe6e868b993e194149d89c359d7fcedfd38c8f8d345d8e"
    end
  end

  on_linux do
    url "https://github.com/daveepope/beanz/releases/download/v0.1.0/beanz-v0.1.0-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "6a90fe0a0b703ef77b35543b5bf733255262dc592b5581664dd225cb79141a55"
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
