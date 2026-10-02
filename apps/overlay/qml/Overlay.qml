import QtQuick

Item {
    id: root
    property string captionText: ""
    property int fontSize: 30

    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: content.height + 32
        visible: root.captionText.length > 0
        radius: 14
        color: "#dd141821"
        border.color: "#30ffffff"
        border.width: 1

        Column {
            id: content
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            anchors.margins: 16
            spacing: 6

            Item {
                width: parent.width
                height: Math.min(caption.implicitHeight, root.fontSize * 1.4 * 4,
                                 Math.max(1, root.height - 32))
                clip: true

                Text {
                    id: caption
                    width: parent.width
                    y: Math.min(0, parent.height - height)
                    text: root.captionText
                    textFormat: Text.PlainText
                    color: "#f5f7fb"
                    font.pixelSize: root.fontSize
                    font.weight: Font.DemiBold
                    wrapMode: Text.WrapAtWordBoundaryOrAnywhere
                    lineHeightMode: Text.FixedHeight
                    lineHeight: root.fontSize * 1.4
                    horizontalAlignment: Text.AlignHCenter
                }
            }
        }
    }
}
