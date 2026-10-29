# Homebrew formula stub for a personal tap.
# Usage (after publishing a release with assets):
#   brew tap arturpanan/macmgr https://github.com/arturpanan/homebrew-macmgr
#   brew install macmgr
#
# Place this file in a tap repo as Formula/macmgr.rb and update url/sha256
# per release. For local smoke-test from source:
#   brew install --build-from-source ./Formula/macmgr.rb

class Macmgr < Formula
  desc "TUI/CLI manager for yabai + skhd + SketchyBar rice"
  homepage "https://github.com/arturpanan/MacOS_Manager"
  version "0.2.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/arturpanan/MacOS_Manager/releases/download/v#{version}/macmgr-macos-aarch64"
      # sha256 "REPLACE_AFTER_RELEASE"
    end
    on_intel do
      url "https://github.com/arturpanan/MacOS_Manager/releases/download/v#{version}/macmgr-macos-x86_64"
      # sha256 "REPLACE_AFTER_RELEASE"
    end
  end

  def install
    bin.install Dir["macmgr-macos-*"].first => "macmgr"
  end

  test do
    assert_match "macmgr", shell_output("#{bin}/macmgr --help")
  end
end
