import sys
import pathlib
import os.path
sys.path.append(os.path.join(pathlib.Path(__file__).parent.parent.parent.parent.resolve(), "xdot.py"))
import xdot.ui

import argparse

import gi
gi.require_version('Gtk', '3.0')
from gi.repository import Gtk

from chart_window import ChartWindow
from tooltip_widget import TooltipWidget
from ros_element import ElementReference, NodeType, ElementType
from r2ta_interface import R2TAInterface

class MyDotWindow(xdot.ui.DotWindow):

    def __init__(self, r2ta: R2TAInterface):
        widget = TooltipWidget(r2ta)
        widget.connect('clicked', self.on_url_clicked)
        
        xdot.ui.DotWindow.__init__(self, widget=widget)
        self.r2ta = r2ta

    def on_url_clicked(self, widget, url: str, event):
        reference = ElementReference.from_ref(url)

        if reference.element_type is not None:
            window = ChartWindow(
                self.r2ta,
                element=reference
            )

            window.show_all()
            return True
        return False


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('tracer_cmd')
    parser.add_argument('source_dir')
    args = parser.parse_args()

    r2ta = R2TAInterface(args.tracer_cmd, args.source_dir)
    window = MyDotWindow(r2ta)
    
    graph = sys.stdin.read()
    window.set_dotcode(graph.encode("utf-8"))
    window.connect('delete-event', Gtk.main_quit)
    Gtk.main()


if __name__ == '__main__':
    main()
