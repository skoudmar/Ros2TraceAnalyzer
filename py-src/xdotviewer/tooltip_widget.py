import os.path
import pathlib
import sys

sys.path.append(
    os.path.join(
        pathlib.Path(__file__)
        .parent.resolve()
        .parent.resolve()
        .parent.resolve()
        .parent.resolve(),
        "xdot.py",
    )
)

import gi
import xdot.ui
from gi.repository import Gtk
from r2ta_interface import R2TAInterface
from ros_element import (
    ChartRequest,
    ChartType,
    ChartValue,
    ElementReference,
)

gi.require_version("Gtk", "3.0")


class TooltipWidget(xdot.ui.DotWidget):
    def __init__(self, r2ta: R2TAInterface):
        xdot.ui.DotWidget.__init__(self)
        self.r2ta = r2ta
        self.prev_element = None
        self.prev_element_managed = False

        self.image = Gtk.Image()
        self.frame = Gtk.Frame()
        self.frame.add(self.image)
        xdot.ui.actions.TooltipContext.add_widget("tooltip_image_frame", self.frame)

    def on_hover(self, element, action, tooltip):
        reference = ElementReference.from_ref(element.tooltip)

        if element.tooltip is not None and reference is not None:
            if self.prev_element != element:
                self.prev_element = element
                image_path = self.r2ta.render(
                    ChartRequest(
                        node=reference.node,
                        value=ChartValue.default_for(
                            reference.element_type, reference.node_type
                        )
                        or ChartValue.CALLBACK_DURATION,
                        plot=ChartType.HISTOGRAM,
                        size=(400, 400),
                    )
                )
                self.image.set_from_pixbuf(image_path)

            self.image.show()
            self.frame.show()

            tooltip.activate()
        else:
            super().on_hover(element, action, tooltip)
