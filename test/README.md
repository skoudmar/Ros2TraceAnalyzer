# Test suite

The test suite uses the [Nix package manager][nix] to install ROS and
other needed dependencies.

If you don't want to compile everything from source, configure ROS
binary cache:

```sh
cat <<EOF >> ~/.config/nix/nix.conf
extra-substituters = https://attic.iid.ciirc.cvut.cz/ros
extra-trusted-public-keys =
ros:JR95vUYsShSqfA1VTYoFt1Nz6uXasm5QrcOsGry9f6Q=
EOF
```

To play with the tests interactively, run:

    nix-shell
    ./tests.bats

To run the tests for a given ROS distro, run:

    nix-shell --argstr distro jazzy --run ./test.bats

Use [niv][] command to update used ROS version, e.g.:

    nix-shell -p niv --command "niv update"

[nix]: https://nixos.org/
[niv]: https://github.com/nmattia/niv
