class Beanz < Formula
  desc "Cognitive debt scoring for AI coding sessions"
  homepage "https://github.com/daveepope/beanz"
  license "MIT"
  version "1.2.0"

  head do
    url "https://github.com/daveepope/beanz.git", branch: "main"
    depends_on "rust" => :build
  end

  on_macos do
    on_arm do
      url "https://github.com/daveepope/beanz/releases/download/v1.2.0/beanz-v1.2.0-aarch64-apple-darwin.tar.gz"
      sha256 "2d7abd72f56723b42344b02bf93137b25f88bd76a9e21b0fad56a1713649275c"
    end
    on_intel do
      url "https://github.com/daveepope/beanz/releases/download/v1.2.0/beanz-v1.2.0-x86_64-apple-darwin.tar.gz"
      sha256 "eadb6a14eeb946df33b3da384ae3424e9f72982e6d60ef082bd3fc2ca1757d62"
    end
  end

  on_linux do
    url "https://github.com/daveepope/beanz/releases/download/v1.2.0/beanz-v1.2.0-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "df3c87ad2d103403130dd41d337ec476de4b6315c683a990d7b94b83104c5e28"
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
