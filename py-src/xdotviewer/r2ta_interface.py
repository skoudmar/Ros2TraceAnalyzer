from ros_element import ChartRequest, ChartValue, ChartType
import subprocess
import tempfile
import os
from pathlib import Path

class R2TAInterface:
    def __init__(self, tracer_cmd, data_dir):
        self.tracer_cmd = tracer_cmd
        self.data_dir = data_dir

    def render(self, chart: ChartRequest) -> str:
        outdir = os.path.join(tempfile.gettempdir(), "r2ta")
        Path(outdir).mkdir(parents=True, exist_ok=True)

        try:
            args = [self.tracer_cmd, "chart", "--element-id", chart.node, '--quantity', chart.value.value, '-i', self.data_dir, '-o', outdir, '--width', str(chart.size[0]), '--height', str(chart.size[1]), chart.plot.value]
            if chart.plot == ChartType.HISTOGRAM and chart.bins is not None:
                args.append("--bins")
                args.append(str(chart.bins))

            tracer = subprocess.run(args, capture_output=True, text=True)
        except subprocess.CalledProcessError as e:
            return None

        return tracer.stdout.strip()

    def export(self, outfile: str, node: str, value: ChartValue):
        try:
            args = [self.tracer_cmd, "extract", '-i', self.data_dir, '-o', outfile, "property", "--element-id", node, '--property', value.value]

            tracer = subprocess.run(args, capture_output=True, text=True)
            print(tracer.stdout)
            print(tracer.stderr)
        except subprocess.CalledProcessError as e:
            return None
