import subprocess
import sys

from gi.repository import GdkPixbuf, Gio, GLib, Gtk
from ros_element import ChartRequest, ChartType, ChartValue


class R2TAInterface:
    def __init__(self, tracer_cmd, data_dir):
        self.tracer_cmd = tracer_cmd
        self.data_dir = data_dir

    def render(self, chart: ChartRequest) -> GdkPixbuf.Pixbuf | None:
        args = [
            self.tracer_cmd,
            "plot",
            chart.value.value,
            chart.node,
            "-i",
            self.data_dir,
            "--size",
            str(chart.size[0]) + "x" + str(chart.size[1]),
            chart.plot.value,
        ]

        if chart.plot == ChartType.HISTOGRAM and chart.bins is not None:
            args.extend(["--bins", str(chart.bins)])

        try:
            plot_process = subprocess.run(
                args, capture_output=True, text=True, check=True
            )
        except subprocess.CalledProcessError as e:
            print(e.stderr, file=sys.stderr)
            return None

        stream = Gio.MemoryInputStream.new_from_bytes(
            GLib.Bytes.new(plot_process.stdout.encode("utf-8"))
        )
        pixbuf = GdkPixbuf.Pixbuf.new_from_stream(stream, None)

        return pixbuf

    def export(self, outfile: str, node: str, value: ChartValue):
        args = [
            self.tracer_cmd,
            "extract",
            "-i",
            self.data_dir,
            "-o",
            outfile,
            "property",
            value.value,
            node,
        ]

        try:
            subprocess.run(args, check=True)
        except subprocess.CalledProcessError as e:
            print(e.stderr, file=sys.stderr)

    def save_as(self, output: str, chart: ChartRequest):
        args = [
            self.tracer_cmd,
            "plot",
            chart.value.value,
            chart.node,
            "-i",
            self.data_dir,
            "-o",
            output,
            "--size",
            str(chart.size[0]) + "x" + str(chart.size[1]),
            chart.plot.value,
        ]

        try:
            subprocess.run(args, check=True)
        except subprocess.CalledProcessError as e:
            print(e.stderr, file=sys.stderr)
