#!/usr/bin/env bash
set -euo pipefail

if rg -n '[[:blank:]]+$' -g '*.md' -g '*.yml' --hidden .; then
    printf 'documentation contains trailing whitespace\n' >&2
    exit 1
fi

perl -MFile::Find -MFile::Basename=dirname -MFile::Spec -e '
    my @bad;
    find(
        {
            no_chdir => 1,
            wanted => sub {
                my $file = $File::Find::name;
                if (-d $file && $file =~ m{^(?:[.]/)?(?:[.]git|target|[.]cache)$}) {
                    $File::Find::prune = 1;
                    return;
                }
                return unless -f $file && $file =~ /[.]md$/;
                open my $fh, q{<}, $file or die qq{$file: $!};
                local $/;
                my $text = <$fh>;
                while ($text =~ m{\[[^]]*\]\(([^)]+)\)}g) {
                    my $target = $1;
                    $target =~ s/^<|>$//g;
                    next if $target eq q{} || $target =~ m{^(?:#|https?://|mailto:)};
                    $target =~ s/#.*$//;
                    my $path = File::Spec->catfile(dirname($file), $target);
                    push @bad, qq{$file: $target} unless -e $path;
                }
            },
        },
        q{.},
    );
    die join(qq{\n}, @bad), qq{\n} if @bad;
'

perl -MFile::Find -e '
    my @bad;
    find(
        {
            no_chdir => 1,
            wanted => sub {
                my $file = $File::Find::name;
                if (-d $file && $file =~ m{^(?:[.]/)?(?:[.]git|target|[.]cache)$}) {
                    $File::Find::prune = 1;
                    return;
                }
                return unless -f $file && $file =~ /[.]md$/;
                open my $fh, q{<}, $file or die qq{$file: $!};
                my ($line_number, $in_fence, $previous_level) = (0, 0, 0);
                while (<$fh>) {
                    $line_number++;
                    if (/^```/) {
                        $in_fence = !$in_fence;
                        next;
                    }
                    next if $in_fence;
                    if (/^(#+) /) {
                        my $level = length($1);
                        push @bad, qq{$file:$line_number heading jumps $previous_level->$level}
                            if $previous_level && $level > $previous_level + 1;
                        $previous_level = $level;
                    }
                }
                push @bad, qq{$file: unbalanced code fence} if $in_fence;
            },
        },
        q{.},
    );
    die join(qq{\n}, @bad), qq{\n} if @bad;
'

printf 'documentation checks passed\n'
