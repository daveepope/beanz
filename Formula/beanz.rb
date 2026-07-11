class Beanz < Formula
  desc "Cognitive debt scoring for AI coding sessions"
  homepage "https://github.com/daveepope/beanz"
  license "MIT"
  version "1.1.0"

  head do
    url "https://github.com/daveepope/beanz.git", branch: "main"
    depends_on "rust" => :build
  end

  on_macos do
    on_arm do
      url "https://github.com/daveepope/beanz/releases/download/v1.1.0/beanz-v1.1.0-aarch64-apple-darwin.tar.gz"
      sha256 "38139fd831380b63c87a5a0c109efa19a31d9c1c0055b252588570dcc8bd0d0e"
    end
    on_intel do
      url "https://github.com/daveepope/beanz/releases/download/v1.1.0/beanz-v1.1.0-x86_64-apple-darwin.tar.gz"
      sha256 "47281f833a1e3c7b84f4bb9cf5e06cfa081907475794e24519b3db8dbc24336a"
    end
  end

  on_linux do
    url "https://github.com/daveepope/beanz/releases/download/v1.1.0/beanz-v1.1.0-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "b66e3e4332e8756a2b88e29d7afc1b7b653e285e3030cfc07e2221994e6e6c10"
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
