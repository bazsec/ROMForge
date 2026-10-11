import Foundation
import CoreGraphics
import ImageIO
import UniformTypeIdentifiers
// swiftc scripts/format-cover.swift -o /tmp/format-cover
// /tmp/format-cover input.png output.jpg 512
// Matches the approved example: 490px art area on a 512px black canvas.
let args=CommandLine.arguments
guard args.count == 4, let requestedSize = Int(args[3]), (16...8192).contains(requestedSize) else {
    fputs("Usage: format-cover INPUT OUTPUT.jpg SIZE\n", stderr)
    exit(1)
}
let input=URL(fileURLWithPath:args[1]), output=URL(fileURLWithPath:args[2])
let size=requestedSize
guard let source=CGImageSourceCreateWithURL(input as CFURL,nil),let image=CGImageSourceCreateImageAtIndex(source,0,nil),let context=CGContext(data:nil,width:size,height:size,bitsPerComponent:8,bytesPerRow:size*4,space:CGColorSpaceCreateDeviceRGB(),bitmapInfo:CGImageAlphaInfo.noneSkipLast.rawValue) else {fatalError("Cannot read image")}
context.setFillColor(CGColor(gray:0,alpha:1));context.fill(CGRect(x:0,y:0,width:size,height:size))
let available=Double(size)*490.0/512.0
let scale=min(available/Double(image.width),available/Double(image.height))
let width=Double(image.width)*scale,height=Double(image.height)*scale
context.interpolationQuality = .high
context.draw(image,in:CGRect(x:(Double(size)-width)/2,y:(Double(size)-height)/2,width:width,height:height))
guard let rendered=context.makeImage(),let destination=CGImageDestinationCreateWithURL(output as CFURL,(output.pathExtension == "jpg" ? UTType.jpeg.identifier : UTType.png.identifier) as CFString,1,nil) else {fatalError("Cannot write image")}
CGImageDestinationAddImage(destination,rendered,[kCGImageDestinationLossyCompressionQuality:0.85] as CFDictionary)
guard CGImageDestinationFinalize(destination) else {fatalError("Image write failed")}
