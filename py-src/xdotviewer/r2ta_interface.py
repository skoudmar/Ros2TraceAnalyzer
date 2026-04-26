import os
import subprocess
import tempfile
from pathlib import Path

from ros_element import ChartRequest, ChartType, ChartValue


class R2TAInterface:
    def __init__(self, tracer_cmd, data_dir):
        self.tracer_cmd = tracer_cmd
        self.data_dir = data_dir

    def render(self, chart: ChartRequest) -> str | None:
        outdir = Path(
            os.path.join(
                tempfile.gettempdir(),
                "r2ta",
            )
        )

        filename = Path(
            f"{chart.plot.value}_{chart.bins}_{chart.size}_{chart.value.value}_{chart.node}.svg"
        )

        full_path = outdir.joinpath(filename)

        if full_path.exists():
            return str(full_path)

        Path(outdir).mkdir(parents=True, exist_ok=True)

        try:
            args = [
                self.tracer_cmd,
                "plot",
                chart.value.value,
                chart.node,
                "-i",
                self.data_dir,
                "-o",
                str(full_path),
                "--size",
                str(chart.size[0]) + "x" + str(chart.size[1]),
                chart.plot.value,
            ]
            if chart.plot == ChartType.HISTOGRAM and chart.bins is not None:
                args.append("--bins")
                args.append(str(chart.bins))

            subprocess.run(args, capture_output=True, text=True)
        except subprocess.CalledProcessError as _:
            return None

        return str(full_path)

    def export(self, outfile: str, node: str, value: ChartValue):
        try:
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

            subprocess.run(args, capture_output=True, text=True)
        except subprocess.CalledProcessError as _:
            return None
