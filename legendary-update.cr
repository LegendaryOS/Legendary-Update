require "colorize"
require "process"
require "io"
require "signal"
require "file_utils"

LOGFILE = "/tmp/legendary-update.log"

FRAMES = [
  "[⋗⋯⋯⋯⋯⋯⋯⋯⋯⋯⋯⋯⋯]", "[ ⋗⋯⋯⋯⋯⋯⋯⋯⋯⋯⋯⋯]", "[  ⋗⋯⋯⋯⋯⋯⋯⋯⋯⋯⋯]",
  "[   ⋗⋯⋯⋯⋯⋯⋯⋯⋯⋯]", "[    ⋗⋯⋯⋯⋯⋯⋯⋯⋯]", "[     ⋗⋯⋯⋯⋯⋯⋯⋯]",
  "[      ⋗⋯⋯⋯⋯⋯⋯]", "[       ⋗⋯⋯⋯⋯⋯]", "[        ⋗⋯⋯⋯⋯]",
  "[         ⋗⋯⋯⋯]", "[          ⋗⋯⋯]", "[           ⋗⋯]"
]

# Banner
def print_banner
  puts "
┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃                LEGENDARY UPDATE                      ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
".colorize(:light_gray)
end

# System info
def system_info
  puts "System Information".colorize(:light_blue).bold
  puts "  Date:     #{Time.local}".colorize(:light_blue)
  puts "  User:     #{ENV["USER"]}".colorize(:light_blue)
  puts "  Kernel:   #{run_capture("uname -r").strip}".colorize(:light_blue)
  puts "  Distro:   #{run_capture("cat /etc/os-release | grep PRETTY_NAME | cut -d= -f2").strip.gsub("\"", "")}".colorize(:light_blue)
  puts
end

# Loading animation
def loading_effect(text : String)
  3.times do
    print "#{text}.".colorize(:light_cyan)
    sleep 0.3; print "."; sleep 0.3; print ".\n"
  end
end

# Status table
def print_status_table(title : String, status : String, type : Symbol)
  frame_top = "┌──────────────────────────────────────────────────┐"
  frame_mid = "├──────────────────────────────────────────────────┤"
  frame_bot = "└──────────────────────────────────────────────────┘"
  color = case type
          when :success then :green
          when :error   then :red
          when :warn    then :yellow
          else          :light_blue
          end
  status_prefix = case type
                  when :success then "Success: "
                  when :error   then "Error:   "
                  when :warn    then "Warning: "
                  else          "Info:    "
                  end
  puts frame_top
  puts "│ #{title.ljust(48)} │".colorize(color)
  puts frame_mid
  puts "│ #{(status_prefix + status).ljust(48)} │".colorize(:white)
  puts frame_bot
  puts
end

# Command output + progress + logging
def run_capture(cmd : String) : String
  output = IO::Memory.new
  error = IO::Memory.new
  status = Process.run(cmd, shell: true, output: output, error: error)
  output.to_s + error.to_s
end

def show_progress(pid : Int32)
  i = 0
  while Process.find(pid)?
    print "\r#{FRAMES[i].colorize(:light_green)}"
    i = (i + 1) % FRAMES.size
    sleep 0.2
  end
  print "\r#{" " * FRAMES[0].size}\r"
end

def run_command(cmd : String, title : String)
  print_status_table(title, "Executing: #{cmd}", :info)
  process = Process.run(cmd, shell: true, output: Process::Redirect::Pipe, error: Process::Redirect::Pipe)
  pid = process.pid
  show_progress(pid)
  process.wait

  File.open(LOGFILE, "a") do |f|
    f.puts "### #{title} @ #{Time.local}"
    f.puts process.output.gets_to_end
    f.puts process.error.gets_to_end
  end

  if process.exit_code == 0
    print_status_table(title, "Completed successfully", :success)
  else
    print_status_table(title, "Failed - Check log: #{LOGFILE}", :error)
  end
end

# Update handlers
def update_pacman
  run_command("sudo pacman -Syu --noconfirm", "Pacman Update")
end

def update_yay
  if run_capture("which yay").strip.empty?
    print_status_table("Yay Update", "yay not installed - Skipping", :warn)
  else
    run_command("yay -Syu --noconfirm", "Yay Update")
  end
end

def update_flatpak
  if run_capture("which flatpak").strip.empty?
    print_status_table("Flatpak Update", "Not installed - Skipping", :warn)
  else
    run_command("flatpak update -y", "Flatpak Update")
  end
end

def update_firmware
  if run_capture("which fwupdmgr").strip.empty?
    print_status_table("Firmware Update", "fwupdmgr not installed - Skipping", :warn)
  else
    run_command("sudo fwupdmgr update", "Firmware Update")
  end
end

def cleanup_pacman
  run_command("sudo pacman -Rns $(pacman -Qdtq)", "Remove Unused Dependencies")
  run_command("sudo pacman -Sc --noconfirm", "Clean Package Cache")
end

# Final menu
def post_menu
  puts "Options".colorize(:light_yellow).bold
  puts "  [E] Exit"
  puts "  [T] Try again"
  puts "  [S] Shutdown"
  puts "  [R] Reboot"
  puts "  [L] Logout"
  print "\nSelect: ".colorize(:light_magenta)
  choice = gets.try &.strip.downcase

  case choice
  when "e"
    puts "Exiting script.".colorize(:light_blue); exit
  when "t"
    main
  when "s"
    run_command("sudo shutdown now", "System Shutdown")
  when "r"
    run_command("sudo reboot", "System Reboot")
  when "l"
    if run_capture("which gnome-session-quit").strip != ""
      run_command("gnome-session-quit --logout --no-prompt", "Session Logout")
    else
      run_command("pkill -KILL -u #{ENV["USER"]}", "Session Termination")
    end
  else
    puts "Invalid option - Exiting.".colorize(:red); exit(1)
  end
end

# Main execution
def main
  Signal::INT.trap { puts "\nInterrupted by user".colorize(:red); exit(130) }
  system("clear")
  print_banner
  system_info
  loading_effect("Preparing update")
  File.open(LOGFILE, "a") { |f| f.puts "\n=== Start: #{Time.local} ===\n" }

  update_pacman
  update_yay
  update_flatpak
  update_firmware
  cleanup_pacman

  File.open(LOGFILE, "a") { |f| f.puts "=== Completed: #{Time.local} ===\n" }
  print_status_table("System Update", "All tasks completed", :success)
  post_menu
end

main
