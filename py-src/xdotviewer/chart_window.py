import os
import os.path
import pathlib

import gi
from gi.repository import Gtk
from r2ta_interface import R2TAInterface
from ros_element import (
    ChartRequest,
    ChartType,
    ChartValue,
    ElementReference,
    ElementType,
    NodeType,
)

gi.require_version("Gtk", "3.0")


class ChartWindow(Gtk.Window):
    def __init__(self, r2ta: R2TAInterface, element: ElementReference):
        self.r2ta = r2ta
        self.element = element

        Gtk.Window.__init__(self)

        vbox = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        self.add(vbox)

        toolbar = Gtk.Toolbar()
        toolbar.set_style(Gtk.ToolbarStyle.ICONS)

        scatter_icon = Gtk.Image.new_from_file(
            os.path.join(
                pathlib.Path(__file__).parent.resolve(), "media", "scatter.png"
            )
        )
        scatter = Gtk.ToolButton(label="Scatter plot", icon_widget=scatter_icon)
        scatter.connect(
            "clicked", lambda w: self.set_and_rerun("chart", ChartType.SCATTER)
        )
        toolbar.insert(scatter, -1)

        histogram_icon = Gtk.Image.new_from_file(
            os.path.join(
                pathlib.Path(__file__).parent.resolve(), "media", "histogram.png"
            )
        )
        histogram = Gtk.ToolButton(label="Histogram", icon_widget=histogram_icon)
        histogram.connect(
            "clicked", lambda w: self.set_and_rerun("chart", ChartType.HISTOGRAM)
        )
        toolbar.insert(histogram, -1)

        histogram_bins = Gtk.SpinButton()
        histogram_bins.set_range(0, 2000)
        histogram_bins.set_increments(1, 10)
        histogram_bins.set_placeholder_text("Bin count")
        histogram_bins.set_width_chars(9)
        histogram_bins.connect(
            "value-changed", lambda w: self.set_and_rerun("bins", w.get_text())
        )

        histogram_bins_item = Gtk.ToolItem()
        histogram_bins_item.add(histogram_bins)
        toolbar.insert(histogram_bins_item, -1)

        if self.element.element_type == ElementType.NODE:
            if self.element.node_type == NodeType.CALLBACK:
                self.value = ChartValue.CALLBACK_DURATION
                callback_duration = Gtk.ToolButton(label="Execution duration")
                callback_duration.connect(
                    "clicked",
                    lambda w: self.set_and_rerun("value", ChartValue.CALLBACK_DURATION),
                )
                toolbar.insert(callback_duration, -1)

                activation_delay = Gtk.ToolButton(label="Activation delay")
                activation_delay.connect(
                    "clicked",
                    lambda w: self.set_and_rerun("value", ChartValue.ACTIVATIONS_DELAY),
                )
                toolbar.insert(activation_delay, -1)
            elif self.element.node_type == NodeType.TIMER:
                self.value = ChartValue.ACTIVATIONS_DELAY
            elif self.element.node_type == NodeType.PUBLISHER:
                self.value = ChartValue.PUBLICATIONS_DELAY
            elif self.element.node_type == NodeType.SUBSCRIBER:
                self.value = ChartValue.MESSAGES_DELAY

        elif self.element.element_type == ElementType.EDGE:
            self.value = ChartValue.MESSAGE_LATENCY

        save_as_icon = Gtk.Image.new_from_icon_name("document-save-as", 16)
        save_as = Gtk.ToolButton(label="Save as", icon_widget=save_as_icon)
        save_as.connect("clicked", self.save_as)
        toolbar.insert(save_as, -1)

        export_as_icon = Gtk.Image.new_from_icon_name("document-save-as", 16)
        export_as = Gtk.ToolButton(label="Export data", icon_widget=save_as_icon)
        export_as.connect("clicked", self.export_as)
        toolbar.insert(export_as, -1)

        self.chart = ChartType.HISTOGRAM
        self.bins = None

        vbox.pack_start(toolbar, False, False, 0)

        image_buffer = self.render()
        self.image = Gtk.Image.new_from_pixbuf(image_buffer)
        vbox.pack_start(self.image, True, True, 0)

    def render(self):
        return self.r2ta.render(
            ChartRequest(
                node=self.element.node,
                value=self.value,
                plot=self.chart,
                bins=self.bins,
            )
        )

    def visualise(self):
        image_buffer = self.render()
        self.image.set_from_pixbuf(image_buffer)

    def set_and_rerun(self, param, value):
        if getattr(self, param) != value:
            setattr(self, param, value)
            self.visualise()

    def export_as(self, w):
        buttons = (
            Gtk.STOCK_CANCEL,
            Gtk.ResponseType.CANCEL,
            Gtk.STOCK_SAVE,
            Gtk.ResponseType.OK,
        )
        chooser = Gtk.FileChooserDialog(
            parent=self,
            title="Export data",
            action=Gtk.FileChooserAction.SAVE,
            buttons=buttons,
        )
        chooser.set_default_response(Gtk.ResponseType.OK)
        chooser.set_current_folder(os.getcwd())

        if chooser.run() == Gtk.ResponseType.OK:
            filename = chooser.get_filename()
            chooser.destroy()
            self.r2ta.export(filename, self.element.node, self.value)
        else:
            chooser.destroy()

    def save_as(self, w):
        default_filter = "PNG image"

        output_formats = {
            "PNG image": "png",
            "SVG image": "svg",
        }
        buttons = (
            Gtk.STOCK_CANCEL,
            Gtk.ResponseType.CANCEL,
            Gtk.STOCK_SAVE,
            Gtk.ResponseType.OK,
        )
        chooser = Gtk.FileChooserDialog(
            parent=self,
            title="Save chart as",
            action=Gtk.FileChooserAction.SAVE,
            buttons=buttons,
        )
        chooser.set_default_response(Gtk.ResponseType.OK)
        chooser.set_current_folder(os.getcwd())

        for name, ext in output_formats.items():
            filter_ = Gtk.FileFilter()
            filter_.set_name(name)
            filter_.add_pattern("*." + ext)
            chooser.add_filter(filter_)
            if name == default_filter:
                chooser.set_filter(filter_)

        if chooser.run() == Gtk.ResponseType.OK:
            filename = chooser.get_filename()
            chooser.destroy()

            self.r2ta.save_as(
                filename,
                ChartRequest(
                    node=self.element.node,
                    value=self.value,
                    plot=self.chart,
                    bins=self.bins,
                ),
            )
        else:
            chooser.destroy()
