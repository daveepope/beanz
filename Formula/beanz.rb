class Beanz < Formula
  desc "Cognitive debt scoring for AI coding sessions"
  homepage "https://github.com/daveepope/beanz"
  license "MIT"
  version "1.0.0"

  head do
    url "https://github.com/daveepope/beanz.git", branch: "main"
    depends_on "rust" => :build
  end

  on_macos do
    on_arm do
      url "https://github.com/daveepope/beanz/releases/download/v1.0.0/beanz-v1.0.0-aarch64-apple-darwin.tar.gz"
      sha256 "03a73575406243682c2033b588013e717b46783a1b50b4e6cb9d3736b619c00b"
    end
    on_intel do
      url "https://github.com/daveepope/beanz/releases/download/v1.0.0/beanz-v1.0.0-x86_64-apple-darwin.tar.gz"
      sha256 "f30d7880886df7cd73f212bdd12c804bda9a28aa04f8c33f6213e780ac9040de"
    end
  end

  on_linux do
    url "https://github.com/daveepope/beanz/releases/download/v1.0.0/beanz-v1.0.0-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "1ea047521620f5884e332a653c1d85a426268e4dbd16d7c2268cbea3a12261d6"
  end

  def install
    if build.head?
      system "cargo", "install", *std_cargo_args
    else
      bin.install "beanz"
    end
  end

  test do
    assert_match "usage:", shell_output("#{bin}/beanz --help", 0)
  end
end
